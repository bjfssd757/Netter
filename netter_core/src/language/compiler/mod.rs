

pub mod vm;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    Nop = 0x00,
    Load = 0x01,
    Store = 0x02,
    Eq = 0x03,
    Add = 0x04,
    Sub = 0x05,
    Mul = 0x06,
    Div = 0x07,
    MakeArray = 0x08,
    PushToArray = 0x09,
    RemoveByIndexFromArray = 0x10,
    GetByIndexFromArray = 0x11,
    FindAndGetFirstElementInArray = 0x12,
    Jmp = 0x13,
    JmpIfFalse = 0x14,
    Call = 0x15,
    Not = 0x16,
    And = 0x17,
    Or = 0x18,
    NotEq = 0x19,
    Less = 0x20,
    Greater = 0x21,
    LessEq = 0x22,
    GreaterEq = 0x23,

    RET = 0xFF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(usize);

pub struct Relocation {
    instruction_addr: usize,
    label: Label,
}

pub struct BytecodeBuilder {
    pub code: Vec<u8>,
    pub constants: Vec<String>,
    labels: Vec<Option<usize>>,
    relocations: Vec<Relocation>,
}

impl BytecodeBuilder {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            labels: Vec::new(),
            relocations: Vec::new(),
        }
    }

    pub fn register_constant(&mut self, value: String) -> u32 {
        if let Some(pos) = self.constants.iter().position(|r| r == &value) {
            pos as u32
        } else {
            self.constants.push(value);
            (self.constants.len() - 1) as u32
        }
    }

    pub fn current_address(&self) -> usize {
        self.code.len()
    }

    pub fn emit_store(&mut self, value: u32) {
        self.emit_op(Opcode::Load);
        self.emit_u32(value);
    }

    pub fn emit_eq(&mut self, a: u32, b: u32) {
        self.emit_op(Opcode::Eq);
        self.emit_u32(a);
        self.emit_u32(b);
    }

    pub fn emit_add(&mut self, a: u32, b: u32) {
        self.emit_op(Opcode::Add);
        self.emit_u32(a);
        self.emit_u32(b);
    }

    pub fn emit_sub(&mut self, a: u32, b: u32) {
        self.emit_op(Opcode::Sub);
        self.emit_u32(a);
        self.emit_u32(b);
    }

    pub fn emit_div(&mut self, a: u32, b: u32) {
        self.emit_op(Opcode::Div);
        self.emit_u32(a);
        self.emit_u32(b);
    }

    pub fn emit_mul(&mut self, a: u32, b: u32) {
        self.emit_op(Opcode::Mul);
        self.emit_u32(a);
        self.emit_u32(b);
    }

    pub fn emit_op(&mut self, op: Opcode) {
        self.code.push(op as u8);
    }

    pub fn emit_u32(&mut self, value: u32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    pub fn make_label(&mut self) -> Label {
        let label = Label(self.labels.len());
        self.labels.push(None);
        label
    }

    pub fn bind_label(&mut self, label: Label) {
        self.labels[label.0] = Some(self.current_address());
    }

    pub fn emit_call(&mut self, obj_idx: u32, method_name: &str, args_count: u32) {
        let str_idx = self.register_constant(method_name.to_string());

        self.emit_op(Opcode::Call);
        self.emit_u32(obj_idx);
        self.emit_u32(str_idx);
        self.emit_u32(args_count);
    }

    pub fn emit_if_false(&mut self, label: Label) {
        self.emit_op(Opcode::JmpIfFalse);
        let target_pos = self.current_address();
        self.emit_u32(0);

        self.relocations.push(Relocation {
            instruction_addr: target_pos,
            label,
        });
    }

    pub fn emit_jmp(&mut self, label: Label) {
        self.emit_op(Opcode::Jmp);
        let target_pos = self.current_address();
        self.emit_u32(0);

        self.relocations.push(Relocation {
            instruction_addr: target_pos,
            label,
        });
    }

    pub fn emit_continue(&mut self, start_label: Label) {
        self.emit_jmp(start_label);
    }

    pub fn emit_break(&mut self, end_label: Label) {
        self.emit_jmp(end_label);
    }

    pub fn emit_make_array(&mut self, elements_count: u32) {
        self.emit_op(Opcode::MakeArray);
        self.emit_u32(elements_count);
    }

    pub fn emit_push_to_array(&mut self, array: u32, item: u32) {
        self.emit_u32(array);
        self.emit_u32(item);
        self.emit_op(Opcode::PushToArray);
    }

    pub fn emit_get_by_index(&mut self, array: u32, idx: u32) {
        self.emit_u32(array);
        self.emit_u32(idx);
        self.emit_op(Opcode::GetByIndexFromArray);
    }

    pub fn emit_remove_by_index(&mut self, array: u32, idx: u32) {
        self.emit_u32(array);
        self.emit_u32(idx);
        self.emit_op(Opcode::RemoveByIndexFromArray);
    }

    pub fn emit_find_first(&mut self, array: u32, item: u32) {
        self.emit_u32(array);
        self.emit_u32(item);
        self.emit_op(Opcode::FindAndGetFirstElementInArray);
    }

    pub fn emit_loop<F>(&mut self, body: F)
    where
        F: FnOnce(&mut Self, Label, Label),
    {
        let start_label = self.make_label();
        let end_label = self.make_label();

        self.bind_label(start_label);

        body(self, start_label, end_label);

        self.emit_jmp(start_label);

        self.bind_label(end_label);
    }

    pub fn emit_while<C, B>(&mut self, condition: C, body: B)
    where
        C: FnOnce(&mut Self),
        B: FnOnce(&mut Self, Label, Label),
    {
        let start_label = self.make_label();
        let end_label = self.make_label();

        self.bind_label(start_label);

        condition(self);

        self.emit_if_false(end_label);

        body(self, start_label, end_label);

        self.emit_jmp(start_label);

        self.bind_label(end_label);
    }

    pub fn link(mut self) -> Vec<u8> {
        for reloc in &self.relocations {
            let target_addr = self.labels[reloc.label.0]
                .expect("Used label which doesn't bind");

            let bytes = (target_addr as u32).to_le_bytes();

            for i in 0..4 {
                self.code[reloc.instruction_addr + i] = bytes[i];
            }
        }
        self.code
    }
}

