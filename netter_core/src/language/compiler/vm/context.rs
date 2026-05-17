use std::collections::HashMap;

use netter_sdk::{Object, RDLTypes};

use crate::language::compiler::Opcode;

const OPCODES_COUNT_YIELD_INTERVAL: u32 = 1000;

pub struct ProcessContext {
    stack: Vec<RDLTypes>,
    variables: HashMap<u32, RDLTypes>,
    objects: Vec<Box<dyn Object>>,
    constants: Vec<String>,
    pc: usize,
}

impl ProcessContext {
    pub fn new(constants: Vec<String>) -> Self {
        Self {
            stack: Vec::new(),
            variables: HashMap::new(),
            constants,
            objects: Vec::new(),
            pc: 0,
        }
    }

    pub fn add_object(&mut self, object: impl Object) {
        self.objects.push(Box::new(object));
    }

    pub fn start(mut self, bytecode: Vec<u8>, operations_limit: u32) -> tokio::task::JoinHandle<Result<Self, String>> {
        tokio::spawn(async move {
            self.pc = 0;
            let mut op_count = 0;

            while self.pc < bytecode.len() {
                let raw_opcode = bytecode[self.pc];
                op_count += 1;

                if op_count > operations_limit {
                    return Err("Runtime error: bytecode operations are out of range".to_string());
                }

                if op_count % OPCODES_COUNT_YIELD_INTERVAL == 0 {
                    tokio::task::yield_now().await;
                }

                let opcode = unsafe {
                    std::mem::transmute::<u8, Opcode>(raw_opcode)
                };

                match opcode {
                    Opcode::Nop => self.pc += 1,

                    Opcode::Load => {
                        self.pc += 1;
                        if self.pc + 4 > bytecode.len() {
                            return Err("Malformed bytecode: missing variable index for load".to_string());
                        }

                        let var_idx = u32::from_le_bytes(bytecode[self.pc..self.pc + 4].try_into().unwrap());
                        self.pc += 4;

                        let value = self.variables.get(&var_idx)
                            .ok_or_else(|| format!("Runtime error: Variable with index {} is not defined", var_idx))?;

                        self.stack.push(value.clone());
                    }

                    Opcode::Store => {
                        self.pc += 1;
                        if self.pc + 4 > bytecode.len() {
                            return Err("Malformed bytecode: missing variable index for Store".to_string());
                        }

                        let var_idx = u32::from_le_bytes(bytecode[self.pc..self.pc + 4].try_into().unwrap());
                        self.pc += 4;

                        let value = self.stack.pop().ok_or("Stack overflow: missing value for Store")?;

                        self.variables.insert(var_idx, value);
                    }

                    Opcode::Jmp => {
                        self.pc += 1;
                        if self.pc + 4 > bytecode.len() {
                            return Err("Malformed bytecode: missing target address for Jmp".to_string());
                        }

                        let target_address = u32::from_le_bytes(bytecode[self.pc..self.pc + 4].try_into().unwrap()) as usize;

                        if target_address > bytecode.len() {
                            return Err(format!("Runtime error: Jmp target {} is out of bytecode bounds", target_address));
                        }

                        self.pc = target_address;
                    }

                    Opcode::JmpIfFalse => {
                        self.pc += 1;
                        if self.pc + 4 > bytecode.len() {
                            return Err("Malformed bytecode: missing target address for JmpIfFalse".to_string());
                        }

                        let target_address = u32::from_le_bytes(bytecode[self.pc..self.pc + 4].try_into().unwrap()) as usize;

                        if target_address > bytecode.len() {
                            return Err(format!("Runtime error: JmpIfFalse target {} is out of bounds", target_address));
                        }

                        let condition_val = self.stack.pop().ok_or("Stack overflow: missing condition for JmpIfFalse")?;

                        let condition = condition_val.as_bool().map_err(|_| "Type error: JmpIfFalse condition must be Boolean".to_string())?;

                        if !condition {
                            self.pc = target_address;
                        } else {
                            self.pc += 4;
                        }
                    }

                    Opcode::MakeArray => {
                        self.pc += 1;

                        if self.pc + 4 > bytecode.len() {
                            return Err("Malformed bytecode: missing count for MakeArray".to_string());
                        }

                        let count = u32::from_le_bytes(bytecode[self.pc..self.pc + 4].try_into().unwrap()) as usize;
                        self.pc += 4;

                        let mut elements = Vec::with_capacity(count);
                        for _ in 0..count {
                            let el = self.stack.pop().ok_or("Stack overflow during MakeArray")?;
                            elements.push(el);
                        }
                        elements.reverse();

                        self.stack.push(RDLTypes::Vector(elements));
                    }

                    Opcode::PushToArray => {
                        self.pc += 1;

                        let item = self.stack.pop().ok_or("Stack overflow: missing item for PushToVector")?;

                        let array = self.stack.pop().ok_or("Stack overflow: missing array for PushToArray")?;

                        let mut val_array = array.as_vec().map_err(|_| "Type error: PushToArray expected a Vector as the target")?;
                        
                        val_array.push(item);

                        self.stack.push(array);
                    }

                    Opcode::GetByIndexFromArray => {
                        self.pc += 1;

                        let idx_val = self.stack.pop().ok_or("Stack overflow: missing index for GetByIndexFromArray")?;
                        let idx = idx_val.as_u64().map_err(|_| "Type error: index must be a Number")? as usize;

                        let array_val = self.stack.pop().ok_or("Stack overflow: missing array for GetByIndexFromArray")?;

                        match array_val {
                            RDLTypes::Vector(vec) => {
                                let element = vec.get(idx)
                                    .ok_or_else(|| format!("Index out of bounds: index is {}, but array length is {}", idx, vec.len()))?;

                                self.stack.push(element.clone());
                            }
                            _ => return Err("Type error: GetByIndexFromArray expected a Vector".to_string()),
                        }
                    }

                    Opcode::RemoveByIndexFromArray => {
                        self.pc += 1;

                        let idx_val = self.stack.pop().ok_or("Stack overflow: missing index for RemoveByIndexFromArray")?;
                        let idx = idx_val.as_u64().map_err(|_| "Type error: index must be a Number")? as usize;

                        let mut array_val = self.stack.pop().ok_or("Stack overflow: missing array for RemoveByIndexFromArray")?;

                        let removed_item = match &mut array_val {
                            RDLTypes::Vector(vec) => {
                                if idx >= vec.len() {
                                    return Err(format!("Index out of bounds for removal from array: index {}, length {}", idx, vec.len()));
                                }
                                vec.remove(idx)
                            }
                            _ => return Err("Type error: RemoveByIndexFromArray expected a Vector".to_string()),
                        };

                        self.stack.push(array_val);

                        self.stack.push(removed_item);
                    }

                    Opcode::FindAndGetFirstElementInArray => {
                        self.pc += 1;

                        let target = self.stack.pop().ok_or("Stack overflow: missing target value for FindAndGetFirstElementInArray")?;

                        let array_val = self.stack.pop().ok_or("Stack overflow: missing array for FindAndGetFirstElementInArray")?;

                        match array_val {
                            RDLTypes::Vector(vec) => {
                                let mut found_element = None;

                                for item in &vec {
                                    if item == &target {
                                        found_element = Some(item.clone());
                                        break;
                                    }
                                }

                                match found_element {
                                    Some(el) => self.stack.push(el),
                                    None => self.stack.push(RDLTypes::Boolean(false)),
                                }
                            }
                            _ => return Err("Type error: FindAndGetFirstElementInArray expected a Vector".to_string()),
                        }
                    }

                    Opcode::And => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for And")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for And")?;
                        
                        let val_a = a.as_bool().map_err(|_| "Type error: argument A is not a Boolean")?;
                        let val_b = b.as_bool().map_err(|_| "Type error: argument B is not a Boolean")?;

                        self.stack.push(RDLTypes::Boolean(val_a && val_b));
                    }

                    Opcode::Or => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for Or")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for Or")?;
                        
                        let val_a = a.as_bool().map_err(|_| "Type error: argument A is not a Boolean")?;
                        let val_b = b.as_bool().map_err(|_| "Type error: argument B is not a Boolean")?;

                        self.stack.push(RDLTypes::Boolean(val_a || val_b));
                    }

                    Opcode::Less => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for Less")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for Less")?;

                        self.stack.push(RDLTypes::Boolean(a < b));
                    }

                    Opcode::LessEq => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for LessEq")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for LessEq")?;

                        self.stack.push(RDLTypes::Boolean(a <= b));
                    }

                    Opcode::Greater => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for Greater")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for Greater")?;

                        self.stack.push(RDLTypes::Boolean(a > b));
                    }

                    Opcode::GreaterEq => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for GreaterEq")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for GreaterEq")?;

                        self.stack.push(RDLTypes::Boolean(a >= b));
                    }

                    Opcode::Not => {
                        self.pc += 1;

                        let a = self.stack.pop().ok_or("Stack overflow: missing argument for Not")?;

                        self.stack.push(RDLTypes::Boolean(!a));
                    }

                    Opcode::NotEq => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for NotEq")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for NotEq")?;

                        self.stack.push(RDLTypes::Boolean(a != b));
                    }

                    Opcode::Eq => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for Eq")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for Eq")?;

                        self.stack.push(RDLTypes::Boolean(a == b));
                    }

                    Opcode::Add => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for Add")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for Add")?;
                        
                        let val_a = a.as_i64().map_err(|_| "Type error: argument A is not a Number")?;
                        let val_b = b.as_i64().map_err(|_| "Type error: argument B is not a Number")?;

                        self.stack.push(RDLTypes::Number(val_a + val_b));
                    }

                    Opcode::Sub => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for Sub")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for Sub")?;
                        
                        let val_a = a.as_i64().map_err(|_| "Type error: argument A is not a Number")?;
                        let val_b = b.as_i64().map_err(|_| "Type error: argument B is not a Number")?;

                        self.stack.push(RDLTypes::Number(val_a - val_b));
                    }

                    Opcode::Div => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for Div")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for Div")?;
                        
                        let val_a = a.as_i64().map_err(|_| "Type error: argument A is not a Number")?;
                        let val_b = b.as_i64().map_err(|_| "Type error: argument B is not a Number")?;

                        self.stack.push(RDLTypes::Number(val_a / val_b));
                    }

                    Opcode::Mul => {
                        self.pc += 1;

                        let b = self.stack.pop().ok_or("Stack overflow: missing first argument for Mul")?;
                        let a = self.stack.pop().ok_or("Stack overflow: missing second argument for Mul")?;
                        
                        let val_a = a.as_i64().map_err(|_| "Type error: argument A is not a Number")?;
                        let val_b = b.as_i64().map_err(|_| "Type error: argument B is not a Number")?;

                        self.stack.push(RDLTypes::Number(val_a * val_b));
                    }

                    Opcode::Call => {
                        self.pc += 1; 

                        if self.pc + 12 > bytecode.len() {
                            return Err("Malformed bytecode: unexpected EOF in Call instruction arguments".to_string());
                        }

                        let obj_idx = u32::from_le_bytes(bytecode[self.pc..self.pc+4].try_into().unwrap()) as usize;
                        self.pc += 4;

                        let const_idx = u32::from_le_bytes(bytecode[self.pc..self.pc + 4].try_into().unwrap()) as usize;
                        self.pc += 4;

                        let args_count = u32::from_le_bytes(bytecode[self.pc..self.pc + 4].try_into().unwrap()) as usize;
                        self.pc += 4;

                        let mut args = Vec::with_capacity(args_count);
                        for _ in 0..args_count {
                            let arg = self.stack.pop().ok_or("Stack underflow while taking arguments for method call")?;
                            args.push(arg);
                        }
                        args.reverse();

                        let method_name = self.constants.get(const_idx)
                            .ok_or_else(|| format!("Constant string not found at index {}", const_idx))?;

                        let object = self.objects.get_mut(obj_idx)
                            .ok_or_else(|| format!("Object not found at index {}", obj_idx))?;

                        let result = object.call_method(method_name, args)?;

                        self.stack.push(result);
                    }

                    Opcode::RET => break,
                    _ => return Err(format!("Unknown or unhandled opcode: 0x{:02X}", raw_opcode)),
                }
            }

            Ok(self)
        })
    }
}