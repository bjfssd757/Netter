#!/usr/bin/env python3
import os
import platform
import subprocess
import sys
import shutil
import re
from pathlib import Path

def is_admin():
    """Check for administrator privileges"""
    if platform.system() == 'Windows':
        try:
            return subprocess.run("net session", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0
        except:
            return False
    else:
        return os.geteuid() == 0

def find_vs_installation_path():
    """Find the Visual Studio installation path"""
    common_paths = [
        r"C:\Program Files\Microsoft Visual Studio",
        r"C:\Program Files (x86)\Microsoft Visual Studio"
    ]
    
    for base_path in common_paths:
        if os.path.exists(base_path):
            versions = [d for d in os.listdir(base_path) if os.path.isdir(os.path.join(base_path, d))]
            if versions:
                for year in ["2022", "2019", "2017"]:
                    if year in versions:
                        editions = os.path.join(base_path, year)
                        for edition in ["Community", "Professional", "Enterprise"]:
                            vs_path = os.path.join(editions, edition)
                            if os.path.exists(vs_path):
                                return vs_path
    
    return None

def find_vcvarsall():
    """Find the vcvarsall.bat file"""
    vs_path = find_vs_installation_path()
    if not vs_path:
        return None
    
    common_vcvarsall_paths = [
        os.path.join(vs_path, "VC", "Auxiliary", "Build", "vcvarsall.bat"),
        os.path.join(vs_path, "VC", "vcvarsall.bat")
    ]
    
    for vcvarsall_path in common_vcvarsall_paths:
        if os.path.exists(vcvarsall_path):
            return vcvarsall_path
    
    for root, dirs, files in os.walk(vs_path):
        if "vcvarsall.bat" in files:
            return os.path.join(root, "vcvarsall.bat")
    
    return None

def setup_msvc_environment():
    """Set up MSVC environment variables"""
    vcvarsall_path = find_vcvarsall()
    
    if not vcvarsall_path:
        print("Could not find vcvarsall.bat. Please ensure that Visual Studio with C++ components is installed.")
        return False
    
    print(f"Found vcvarsall.bat: {vcvarsall_path}")
    print("Setting up MSVC environment variables...")
    
    architecture = "x64" if platform.architecture()[0] == "64bit" else "x86"
    try:
        cmd = f'"{vcvarsall_path}" {architecture} && set'
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        
        if result.returncode != 0:
            print(f"Error running vcvarsall.bat: {result.stderr}")
            return False
        
        env_vars = {}
        for line in result.stdout.splitlines():
            if '=' in line:
                name, value = line.split('=', 1)
                env_vars[name] = value
        
        for name, value in env_vars.items():
            if name.upper() in ["PATH", "INCLUDE", "LIB", "LIBPATH"]:
                os.environ[name] = value
        
        print("MSVC environment variables successfully set")
        
        cl_check = subprocess.run("where cl", shell=True, capture_output=True, text=True)
        if cl_check.returncode == 0:
            print(f"MSVC compiler found: {cl_check.stdout.strip()}")
            return True
        else:
            print("Warning: Could not find cl compiler even after setting environment variables")
            
            cl_paths = []
            for root, dirs, files in os.walk(os.path.dirname(os.path.dirname(vcvarsall_path))):
                if "cl.exe" in files:
                    cl_paths.append(os.path.join(root, "cl.exe"))
            
            if cl_paths:
                print(f"Found the following cl.exe instances:")
                for cl_path in cl_paths:
                    print(f"  - {cl_path}")
                
                cl_dir = os.path.dirname(cl_paths[0])
                os.environ["PATH"] = cl_dir + os.pathsep + os.environ["PATH"]
                print(f"Added path to PATH: {cl_dir}")
                return True
    except Exception as e:
        print(f"Error setting up MSVC environment variables: {e}")
    
    return False

def check_command_exists(command):
    """Check if a command exists in the system PATH"""
    if command == 'cl' and platform.system() == 'Windows':
        if setup_msvc_environment():
            return True
        else:
            return False
    
    if platform.system() == 'Windows':
        check_cmd = f'where {command}'
    else:
        check_cmd = f'which {command}'
    
    try:
        result = subprocess.run(check_cmd, shell=True, capture_output=True, text=True)
        return result.returncode == 0
    except:
        return False

def get_qt_version():
    """Get installed Qt version"""
    try:
        qmake_cmds = ['qmake6 -v', 'qmake -v']
        
        for qmake_cmd in qmake_cmds:
            result = subprocess.run(qmake_cmd, shell=True, capture_output=True, text=True)
            if result.returncode == 0:
                output = result.stdout
                if 'Qt version' in output:
                    version_line = [line for line in output.splitlines() if 'Qt version' in line][0]
                    version = version_line.split('Qt version')[1].strip()
                    if version.startswith('6.'):
                        return version
    except:
        pass
    return None

def get_latest_qt_version():
    """Get the latest available Qt 6 version"""
    try:
        print("Determining the latest available Qt 6.x version...")
        cmd = 'aqt list-qt windows desktop'
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        
        if result.returncode == 0:
            output = result.stdout
            qt6_versions = []
            for line in output.splitlines():
                match = re.search(r'6\.\d+\.\d+', line)
                if match:
                    qt6_versions.append(match.group(0))
            
            if qt6_versions:
                qt6_versions.sort(key=lambda s: [int(u) for u in s.split('.')])
                latest_version = qt6_versions[-1]
                print(f"Found latest available Qt version: {latest_version}")
                return latest_version
            else:
                print("No Qt 6.x versions found. Using Qt 6.5.3 by default.")
                return "6.5.3"
        else:
            print("Failed to get list of Qt versions. Using Qt 6.5.3 by default.")
            return "6.5.3"
    except Exception as e:
        print(f"Error getting list of Qt versions: {e}")
        return "6.5.3"

def get_qt_architecture(qt_version):
    """Get available architecture for Qt"""
    try:
        print(f"Determining available architecture for Qt {qt_version}...")
        cmd = f'aqt list-qt windows desktop --arch {qt_version}'
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        
        if result.returncode == 0:
            output = result.stdout
            if "win64_msvc2019_64" in output:
                return "win64_msvc2019_64"
            elif "win64_mingw" in output:
                return "win64_mingw"
            elif "win64_msvc2015_64" in output:
                return "win64_msvc2015_64"
            elif "win64_msvc2017_64" in output:
                return "win64_msvc2017_64"
            else:
                print("Could not find suitable architecture for MSVC. Using win64_msvc2019_64 by default.")
                return "win64_msvc2019_64"
        else:
            print("Failed to get list of architectures. Using win64_msvc2019_64 by default.")
            return "win64_msvc2019_64"
    except Exception as e:
        print(f"Error getting list of architectures: {e}")
        return "win64_msvc2019_64"

def get_qt_modules(qt_version, architecture):
    """Get available modules for Qt"""
    try:
        print(f"Determining available modules for Qt {qt_version} ({architecture})...")
        cmd = f'aqt list-qt windows desktop --modules {qt_version} {architecture}'
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        
        if result.returncode == 0:
            output = result.stdout
            modules = []
            
            if "qtbase" in output.lower():
                modules.append("qtbase")
            if "qt5compat" in output.lower():
                modules.append("qt5compat")
            
            if not modules:
                print("Could not determine modules. Using basic modules.")
                return "qtbase qt5compat"
            
            return " ".join(modules)
        else:
            print("Failed to get list of modules. Using basic modules.")
            return "qtbase qt5compat"
    except Exception as e:
        print(f"Error getting list of modules: {e}")
        return "qtbase qt5compat"

def install_dependencies_windows():
    """Install dependencies for Windows"""
    print("Checking dependencies for Windows...")
    
    vs_installed = check_command_exists('cl')
    if not vs_installed:
        print("MSVC (Visual Studio) not found. Please install Visual Studio with C++ development tools.")
        print("You can download it from: https://visualstudio.microsoft.com/downloads/")
        print("Make sure that 'Desktop development with C++' component is selected")
        input("Press Enter after installing Visual Studio to continue...")
        
        vs_installed = check_command_exists('cl')
        if not vs_installed:
            print("MSVC still not found. Setting up environment manually...")
            setup_msvc_environment()
    else:
        print("✓ MSVC found!")
    
    cmake_installed = check_command_exists('cmake')
    if not cmake_installed:
        print("Installing CMake...")
        subprocess.run('pip install cmake', shell=True, check=True)
    else:
        print("✓ CMake found!")
    
    ninja_installed = check_command_exists('ninja')
    if not ninja_installed:
        print("Installing Ninja...")
        subprocess.run('pip install ninja', shell=True, check=True)
    else:
        print("✓ Ninja found!")
    
    qt_version = get_qt_version()
    target_qt_version = "6.5.3"
    
    if qt_version and qt_version.startswith('6.'):
        print(f"✓ Found installed Qt version {qt_version}!")
    else:
        print("Qt 6 not found, installing latest available version...")
        
        subprocess.run('pip install aqtinstall', shell=True, check=True)
        
        target_qt_version = get_latest_qt_version()
        
        architecture = get_qt_architecture(target_qt_version)
        
        modules = get_qt_modules(target_qt_version, architecture)
        
        qt_install_path = os.path.join(os.path.expanduser('~'), 'Qt')
        print(f"Installing Qt {target_qt_version} in {qt_install_path}...")
        
        cmd = f'aqt install-qt windows desktop {target_qt_version} {architecture} -O "{qt_install_path}" --modules {modules}'
        print(f"Executing command: {cmd}")
        subprocess.run(cmd, shell=True, check=True)
        
        qt_bin_path = os.path.join(qt_install_path, target_qt_version, architecture.split('_')[0] + '_' + architecture.split('_')[1], 'bin')
        os.environ["PATH"] += os.pathsep + qt_bin_path
        
        print("Adding Qt to system PATH...")
        if is_admin():
            path_cmd = f'setx /M PATH "%PATH%;{qt_bin_path}"'
            subprocess.run(path_cmd, shell=True)
        else:
            path_cmd = f'setx PATH "%PATH%;{qt_bin_path}"'
            subprocess.run(path_cmd, shell=True)
            print("Note: Qt added only to user PATH. Run as administrator to add to system PATH.")

def install_dependencies_linux():
    """Install dependencies for Linux"""
    print("Checking dependencies for Linux...")
    
    mingw_installed = check_command_exists('g++')
    if not mingw_installed:
        print("Installing g++...")
        subprocess.run('sudo apt-get update && sudo apt-get install -y g++', shell=True, check=True)
    else:
        print("✓ g++ found!")
    
    cmake_installed = check_command_exists('cmake')
    if not cmake_installed:
        print("Installing CMake...")
        subprocess.run('sudo apt-get update && sudo apt-get install -y cmake', shell=True, check=True)
    else:
        print("✓ CMake found!")
    
    ninja_installed = check_command_exists('ninja')
    if not ninja_installed:
        print("Installing Ninja...")
        subprocess.run('sudo apt-get update && sudo apt-get install -y ninja-build', shell=True, check=True)
    else:
        print("✓ Ninja found!")
    
    qt_version = get_qt_version()
    
    if qt_version and qt_version.startswith('6.'):
        print(f"✓ Found installed Qt version {qt_version}!")
    else:
        print("Qt 6 not found, installing via system packages...")
        
        try:
            print("Installing Qt6 via apt...")
            subprocess.run('sudo apt-get update', shell=True, check=True)
            subprocess.run('sudo apt-get install -y qt6-base-dev qt6-declarative-dev qt6-tools-dev-tools', shell=True, check=True)
            
            qt_version = get_qt_version()
            if qt_version and qt_version.startswith('6.'):
                print(f"✓ Successfully installed Qt {qt_version} via system packages!")
            else:
                print("Warning: Qt installation via apt may not have succeeded")
                
        except Exception as e:
            print(f"Error installing Qt via apt: {e}")
            print("Creating a virtual environment for aqtinstall...")
            
            subprocess.run('sudo apt-get install -y python3-venv', shell=True, check=True)
            
            venv_path = os.path.join(os.path.expanduser('~'), '.venv-qt-installer')
            subprocess.run(f'python3 -m venv {venv_path}', shell=True, check=True)
            
            subprocess.run(f'{venv_path}/bin/pip install aqtinstall', shell=True, check=True)
            
            cmd = f'{venv_path}/bin/aqt list-qt linux desktop'
            result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
            
            target_qt_version = "6.5.3"
            
            if result.returncode == 0:
                output = result.stdout
                qt6_versions = []
                for line in output.splitlines():
                    match = re.search(r'6\.\d+\.\d+', line)
                    if match:
                        qt6_versions.append(match.group(0))
                
                if qt6_versions:
                    qt6_versions.sort(key=lambda s: [int(u) for u in s.split('.')])
                    target_qt_version = qt6_versions[-1]
                    print(f"Found latest available Qt version: {target_qt_version}")
            
            qt_install_path = os.path.join(os.path.expanduser('~'), 'Qt')
            print(f"Installing Qt {target_qt_version} in {qt_install_path}...")
            
            cmd = f'{venv_path}/bin/aqt install-qt linux desktop {target_qt_version} gcc_64 -O "{qt_install_path}" --modules qtbase qt5compat'
            subprocess.run(cmd, shell=True, check=True)
            
            qt_bin_path = os.path.join(qt_install_path, target_qt_version, 'gcc_64', 'bin')
            os.environ["PATH"] += os.pathsep + qt_bin_path
            
            bashrc_path = os.path.join(os.path.expanduser('~'), '.bashrc')
            with open(bashrc_path, 'a') as f:
                f.write(f'\n# Added by Netter installer\nexport PATH="$PATH:{qt_bin_path}"\n')
                
            print(f"Qt added to PATH in {bashrc_path}")

def update_cmake_file():
    """Update CMake file with automatic deployment settings"""
    cmake_path = "CMakeLists.txt"
    
    encodings = ['utf-8', 'utf-16', 'latin-1', 'cp1251']
    
    cmake_content = None
    for encoding in encodings:
        try:
            with open(cmake_path, 'r', encoding=encoding) as f:
                cmake_content = f.read()
                print(f"CMakeLists.txt file successfully read with encoding {encoding}")
                break
        except UnicodeDecodeError:
            continue
    
    if cmake_content is None:
        print("Error: Failed to read CMakeLists.txt file. Check its encoding.")
        return
    
    if "WINDEPLOYQT_EXECUTABLE" in cmake_content:
        print("CMake file already updated!")
        return
    
    last_target_link = cmake_content.rfind('target_link_libraries')
    if last_target_link == -1:
        print("Could not find suitable place in CMakeLists.txt for updating!")
        return
    
    end_pos = cmake_content.find(')', last_target_link)
    if end_pos == -1:
        print("Could not find end of target_link_libraries section!")
        return
    
    first_part = cmake_content[:end_pos+1]
    
    deployment_pos = cmake_content.rfind('add_custom_command')
    if deployment_pos != -1:
        deployment_end = cmake_content.find(')', deployment_pos)
        if deployment_end != -1:
            second_part = cmake_content[deployment_end+1:]
        else:
            second_part = cmake_content[end_pos+1:]
    else:
        second_part = cmake_content[end_pos+1:]
    
    deployment_code = """

if(WIN32)
    
    find_program(WINDEPLOYQT_EXECUTABLE windeployqt HINTS "${CMAKE_PREFIX_PATH}/bin")
    if(WINDEPLOYQT_EXECUTABLE)
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND ${WINDEPLOYQT_EXECUTABLE} --no-translations --no-system-d3d-compiler "$<TARGET_FILE:${PROJECT_NAME}>"
            COMMENT "Running windeployqt to copy Qt dependencies..."
        )
    else()
        message(WARNING "windeployqt not found, Qt dependencies will not be automatically copied")
    endif()
elseif(UNIX AND NOT APPLE)
    
    find_program(LINUXDEPLOYQT_EXECUTABLE linuxdeployqt HINTS "${CMAKE_PREFIX_PATH}/bin")
    if(LINUXDEPLOYQT_EXECUTABLE)
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND ${LINUXDEPLOYQT_EXECUTABLE} "$<TARGET_FILE:${PROJECT_NAME}>" -always-overwrite -no-translations
            COMMENT "Running linuxdeployqt to copy Qt dependencies..."
        )
    else()
        message(STATUS "linuxdeployqt not found, attempting to use qt-deploy")
        
        
        file(WRITE "${CMAKE_BINARY_DIR}/qt-deploy.sh"
            "#!/bin/bash\\n"
            "echo 'Copying Qt dependencies...'\\n"
            "EXECUTABLE=\\"$<TARGET_FILE:${PROJECT_NAME}>\\"\\n"
            "DEST_DIR=\\"$(dirname \\"$EXECUTABLE\\")\\"\\n"
            "ldd \\"$EXECUTABLE\\" | grep -i qt | awk '{print $3}' | xargs -I{} cp -v {} \\"$DEST_DIR\\"\\n"
        )
        execute_process(COMMAND chmod +x "${CMAKE_BINARY_DIR}/qt-deploy.sh")
        
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND "${CMAKE_BINARY_DIR}/qt-deploy.sh"
            COMMENT "Running qt-deploy script to copy Qt dependencies..."
        )
    endif()
endif()
"""
    
    updated_cmake = first_part + deployment_code + second_part
    
    try:
        with open(cmake_path, 'w', encoding='utf-8') as f:
            f.write(updated_cmake)
        print("CMake file updated with automatic deployment settings!")
    except Exception as e:
        print(f"Error writing CMakeLists.txt file: {e}")

def create_startup_script():
    """Create batch/shell script to add netter to PATH and build/run the application"""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    build_dir = os.path.join(script_dir, 'build')
    netter_path = os.path.join(script_dir, 'target', 'release')
    netter_exe = os.path.join(netter_path, 'netter.exe' if platform.system() == 'Windows' else 'netter')
    
    if platform.system() == 'Windows':
        script_path = os.path.join(script_dir, 'run_netter.bat')
        
        script_content = f"""@echo off
:: Set up MSVC environment
echo Setting up MSVC environment...
set "VCVARSALL_PATH={find_vcvarsall() or 'NOT_FOUND'}"
if not "%VCVARSALL_PATH%"=="NOT_FOUND" (
    call "%VCVARSALL_PATH%" x64
    echo MSVC environment set up successfully
) else (
    echo WARNING: Could not find vcvarsall.bat, MSVC environment variables may not be set up
)

:: Add netter to PATH
echo Adding {netter_path} to PATH...
setx PATH "%PATH%;{netter_path}"

:: Check if build directory exists
if not exist "{build_dir}" (
    echo Creating build directory and building project...
    mkdir "{build_dir}"
    cd "{build_dir}"
    cmake .. -G "Ninja"
    ninja
) else (
    echo Build directory found
)

:: Check if executable file exists and run
if exist "{build_dir}\\Netter.exe" (
    echo Running Netter...
    "{build_dir}\\Netter.exe"
) else (
    echo Building Netter...
    cd "{build_dir}"
    cmake .. -G "Ninja"
    ninja
    
    :: Run executable if build successful
    if exist "{build_dir}\\Netter.exe" (
        echo Running Netter...
        "{build_dir}\\Netter.exe"
    ) else (
        echo Failed to build Netter
        pause
    )
)
"""
    else:
        script_path = os.path.join(script_dir, 'run_netter.sh')
        
        script_content = f"""#!/bin/bash
# Add netter to PATH
echo "Adding {netter_path} to PATH..."
export PATH="$PATH:{netter_path}"

# Add to .bashrc for persistence
if ! grep -q "{netter_path}" ~/.bashrc; then
    echo 'export PATH="$PATH:{netter_path}"' >> ~/.bashrc
    echo "PATH updated in .bashrc file"
fi

# Check if build directory exists
if [ ! -d "{build_dir}" ]; then
    echo "Creating build directory and building project..."
    mkdir -p "{build_dir}"
    cd "{build_dir}"
    cmake .. -G "Ninja"
    ninja
else
    echo "Build directory found"
fi

# Check if executable file exists and run
if [ -f "{build_dir}/Netter" ]; then
    echo "Running Netter..."
    "{build_dir}/Netter"
else
    echo "Building Netter..."
    cd "{build_dir}"
    cmake .. -G "Ninja"
    ninja
    
    # Run executable if build successful
    if [ -f "{build_dir}/Netter" ]; then
        echo "Running Netter..."
        "{build_dir}/Netter"
    else
        echo "Failed to build Netter"
    fi
fi
"""
    
    with open(script_path, 'w') as f:
        f.write(script_content)
    
    if platform.system() != 'Windows':
        os.chmod(script_path, 0o755)
    
    print(f"Created startup script: {script_path}")
    return script_path

def main():
    print("Setting up dependencies for Netter project...")
    
    if platform.system() == 'Windows':
        install_dependencies_windows()
    else:
        install_dependencies_linux()
    
    update_cmake_file()
    
    script_path = create_startup_script()
    
    print("\nSetup complete!")
    print(f"To build and run Netter, execute the script: {script_path}")
    
    print("\nStarting Netter...")
    try:
        if platform.system() == 'Windows':
            subprocess.run(script_path, shell=True)
        else:
            subprocess.run(f"bash {script_path}", shell=True)
    except Exception as e:
        print(f"Error running script: {e}")
        print("Please run the script manually.")

if __name__ == "__main__":
    main()