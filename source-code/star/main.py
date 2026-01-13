import re
import sys
import os
import subprocess
import hcl2
import shutil

class HackerCompiler:
    def __init__(self, config_path="Virus.hcl"):
        self.cache_dir = os.path.abspath("cache")
        self.venv_dir = os.path.join(self.cache_dir, "env")
        self.build_dir = os.path.join(self.cache_dir, "build")
        self.src_dir = os.path.join(self.build_dir, "src")
        self.cmd_dir = "/cmd"

        self.home_dir = os.path.expanduser("~")
        self.lib_repo_paths = {
            "virus": os.path.join(self.home_dir, ".HackerScript/libs/virus/"),
            "vira": os.path.join(self.home_dir, ".HackerScript/libs/vira/")
        }

        self.config = self._load_config(config_path)
        project_list = self.config.get('project', [{}])
        project_cfg = project_list[0] if isinstance(project_list, list) else project_list
        self.project_name = project_cfg.get('name', 'virus_payload')

        self.external_libs = []
        self.pip_requirements = {"stackprinter"}

    def _load_config(self, path):
        if not os.path.exists(path): return {}
        try:
            with open(path, 'r', encoding='utf-8') as f:
                return hcl2.load(f)
        except: return {}

    def setup_workspace(self):
        if not os.path.exists(self.cache_dir):
            os.makedirs(self.cache_dir)
        os.makedirs(self.src_dir, exist_ok=True)
        if not os.path.exists(self.venv_dir):
            print(f"[*] Tworzenie izolowanego środowiska Python w {self.venv_dir}...")
            subprocess.run([sys.executable, "-m", "venv", self.venv_dir], check=True)
        self._run_pip(["install", "--upgrade", "pip"])

    def _run_pip(self, args):
        pip_exe = os.path.join(self.venv_dir, "Scripts", "pip") if os.name == "nt" else os.path.join(self.venv_dir, "bin", "pip")
        subprocess.run([pip_exe] + args, check=True)

    def translate_hcs_to_python(self, hcs_path):
        if not os.path.exists(hcs_path):
            raise FileNotFoundError(f"Brak pliku: {hcs_path}")
        with open(hcs_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        processed_lines = []
        header_imports = [
            "import subprocess",
            "import sys",
            "import os",
            "import stackprinter",
            "stackprinter.set_excepthook(style='darkbg2')"
        ]

        indent_level = 0
        in_sh_block = False

        for line in lines:
            raw_line = line.strip()
            raw_line = re.sub(r'@.*', '', raw_line).strip()
            if not raw_line and not in_sh_block: continue

            if raw_line == "--- manual ---":
                self.pip_requirements.add("numpy")
                header_imports.append("import numpy")
                header_imports.append("import ctypes")
                continue

            if "memory" in raw_line.lower() or "alloc" in raw_line.lower():
                processed_lines.append(f"{'    ' * indent_level}# Memory Management: {raw_line}")
                continue

            require_match = re.match(r'require\s+"([^"]+)"', raw_line)
            if require_match:
                req_file = require_match.group(1)
                req_path = os.path.join(self.cmd_dir, req_file)
                if os.path.exists(req_path):
                    with open(req_path, 'r') as rf:
                        processed_lines.append(rf.read())
                continue

            lib_match = re.match(r'import <(virus|vira):(\w+)>', raw_line)
            if lib_match:
                repo, lib_name = lib_match.groups()
                lib_file = os.path.join(self.lib_repo_paths[repo], f"{lib_name}.a")
                if os.path.exists(lib_file):
                    self.external_libs.append(lib_file)
                    header_imports.append(f"import {lib_name}")
                continue

            if raw_line == "sh [":
                in_sh_block = True
                continue
            if in_sh_block:
                if raw_line == "]":
                    in_sh_block = False
                    continue
                processed_lines.append(f"{'    ' * indent_level}subprocess.run(f\"{raw_line}\", shell=True, check=True)")
                continue

            raw_line = re.sub(r'(import\s+)?<core:([\w\.]+)>', r'import \2', raw_line)
            if raw_line.startswith('func '): raw_line = raw_line.replace('func ', 'def ', 1)
            if raw_line.startswith('log '): raw_line = f"print({raw_line[4:].strip()})"
            if raw_line.startswith('] except'):
                indent_level -= 1
                processed_lines.append("    "*indent_level + raw_line[1:].strip().replace('[', ':'))
                indent_level += 1
                continue
            if raw_line.startswith('] else'):
                indent_level -= 1
                processed_lines.append("    "*indent_level + "else:")
                indent_level += 1
                continue
            if raw_line == ']':
                indent_level -= 1
                continue
            current_indent = "    " * max(0, indent_level)
            if raw_line.endswith('['):
                processed_lines.append(current_indent + raw_line[:-1].rstrip() + ":")
                indent_level += 1
            else:
                processed_lines.append(current_indent + raw_line)

        print(f"[*] Instalacja zależności: {self.pip_requirements}")
        self._run_pip(["install"] + list(self.pip_requirements))
        return "\n".join(dict.fromkeys(header_imports)) + "\n\n" + "\n".join(processed_lines)

    def generate_build_rs(self):
        linker_instructions = []
        for lib_path in self.external_libs:
            dir_name = os.path.dirname(lib_path)
            file_name = os.path.basename(lib_path)
            lib_name = file_name.replace("lib", "").replace(".a", "")
            linker_instructions.append(f'println!("cargo:rustc-link-search=native={dir_name}");')
            linker_instructions.append(f'println!("cargo:rustc-link-lib=static={lib_name}");')

        content = f"fn main() {{\n    {chr(10).join(linker_instructions)}\n    println!(\"cargo:rerun-if-changed=build.rs\");\n}}"
        with open(os.path.join(self.build_dir, "build.rs"), "w") as f: f.write(content)

    def prepare_cargo_project(self, is_lib):
        lib_section = f'[lib]\nname = "{self.project_name}"\ncrate-type = ["staticlib"]' if is_lib else ""
        cargo_toml = f"""
[package]
name = "{self.project_name}"
version = "0.1.0"
edition = "2021"
build = "build.rs"

{lib_section}

[dependencies]
pyo3 = {{ version = "0.23.5", features = ["auto-initialize"] }}
"""
        with open(os.path.join(self.build_dir, "Cargo.toml"), "w") as f: f.write(cargo_toml)

    def _get_site_packages_path(self):
        if os.name == "nt":
            return os.path.join(self.venv_dir, "Lib", "site-packages").replace("\\", "\\\\")
        python_version = f"python{sys.version_info.major}.{sys.version_info.minor}"
        return os.path.join(self.venv_dir, "lib", python_version, "site-packages")

    def _get_rust_bin_template(self):
        sp_path = self._get_site_packages_path()
        return f"""
use pyo3::prelude::*;
use pyo3::types::{{PyList, PyString}};
use std::env;
use std::ffi::CString;

fn main() -> PyResult<()> {{
    Python::with_gil(|py| {{
        let sys = py.import("sys")?;
        let site_packages = "{sp_path}";

        let binding = sys.getattr("path")?;
        let path: &Bound<'_, PyList> = binding.downcast()?;
        path.insert(0, site_packages)?;

        let args: Vec<String> = env::args().collect();
        // POPRAWKA E0277: PyList::new zwraca PyResult, musimy użyć '?'
        let py_args = PyList::new(py, &args)?;
        sys.setattr("argv", py_args)?;

        let code_raw = include_str!("../logic.py");
        // POPRAWKA E0308: Konwersja kodu na CString dla py.run
        let code = CString::new(code_raw).unwrap();

        if let Err(e) = py.run(&code, None, None) {{
             eprintln!("--- HCS RUNTIME FATAL ERROR ---");
             e.print(py);
        }}
        Ok(())
    }})
}}
"""

    def _get_rust_lib_template(self):
        sp_path = self._get_site_packages_path()
        return f"""
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::ffi::CString;

#[no_mangle]
pub extern "C" fn hcs_init() {{
    Python::with_gil(|py| {{
        if let Ok(sys) = py.import("sys") {{
            let _ = sys.getattr("path").and_then(|p| {{
                let path: &Bound<'_, PyList> = p.downcast().map_err(|_| PyErr::fetch(py))?;
                path.insert(0, "{sp_path}")
            }});
        }}
        let code_raw = include_str!("../logic.py");
        let code = CString::new(code_raw).unwrap();
        let _ = py.run(&code, None, None);
    }});
}}
"""

    def build_cargo(self):
        python_exe = os.path.join(self.venv_dir, "Scripts", "python.exe") if os.name == "nt" else os.path.join(self.venv_dir, "bin", "python")
        os.environ["PYTHON_SYS_EXECUTABLE"] = python_exe
        res = subprocess.run(["cargo", "build", "--release"], cwd=self.build_dir)
        if res.returncode == 0:
            print(f"[+] Sukces! Binarka w {os.path.join(self.build_dir, 'target/release/')}")
        else:
            print("[-] BŁĄD Cargo.")

    def run(self, input_hcs, is_lib=False):
        print(f"[*] Kompilacja HackerScript z DEBUG (stackprinter): {input_hcs}")
        self.setup_workspace()
        py_code = self.translate_hcs_to_python(input_hcs)
        with open(os.path.join(self.build_dir, "logic.py"), "w", encoding='utf-8') as f: f.write(py_code)
        self.generate_build_rs()
        self.prepare_cargo_project(is_lib)
        rust_template = self._get_rust_lib_template() if is_lib else self._get_rust_bin_template()
        rust_file = "lib.rs" if is_lib else "main.rs"
        with open(os.path.join(self.src_dir, rust_file), "w") as f: f.write(rust_template)
        self.build_cargo()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Użycie: python main.py <plik.hcs> [--lib]")
        sys.exit(0)
    HackerCompiler().run(sys.argv[1], "--lib" in sys.argv)
