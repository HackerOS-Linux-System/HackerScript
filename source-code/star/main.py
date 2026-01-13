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
        for req in self.pip_requirements:
            self._run_pip(["install", req])

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
            "import subprocess", "import sys", "import os", "import shutil",
            "import stackprinter", "stackprinter.set_excepthook(style='darkbg2')"
        ]

        indent_level = 0
        in_sh_block = False

        for line in lines:
            raw_line = line.strip()
            raw_line = re.sub(r'@.*', '', raw_line).strip()
            if not raw_line and not in_sh_block: continue

            # 1. Obsługa JEDNOLINIOWEGO sh [ ... ]
            if raw_line.startswith("sh [") and raw_line.endswith("]"):
                content = raw_line[4:-1].strip()
                if "{" in content and "val_" not in content:
                    content = content.replace("{", "{{").replace("}", "}}")
                content = re.sub(r'val_(\w+)', r'{\1}', content)
                indent = '    ' * indent_level
                processed_lines.append(f'{indent}subprocess.run(f"""{content}""", shell=True, check=True)')
                continue

            # 2. Obsługa WIELOLINIOWEGO sh [
            if raw_line == "sh [":
                in_sh_block = True
                continue

            if in_sh_block:
                if raw_line == "]":
                    in_sh_block = False
                    continue
                content = raw_line
                if "{" in content and "val_" not in content:
                    content = content.replace("{", "{{").replace("}", "}}")
                content = re.sub(r'val_(\w+)', r'{\1}', content)
                indent = '    ' * indent_level
                processed_lines.append(f'{indent}subprocess.run(f"""{content}""", shell=True, check=True)')
                continue

            # 3. Tłumaczenie słów kluczowych
            raw_line = re.sub(r'(import\s+)?<core:([\w\.]+)>', r'import \2', raw_line)
            if raw_line.startswith('func '): raw_line = raw_line.replace('func ', 'def ', 1)
            if raw_line.startswith('log '): raw_line = f"print({raw_line[4:].strip()})"

            # 4. Obsługa JEDNOLINIOWYCH IF-ów (np. if cond [ action ])
            if raw_line.startswith("if ") and raw_line.count("[") == 1 and raw_line.endswith("]"):
                condition = raw_line[3:raw_line.find("[")].strip()
                action = raw_line[raw_line.find("[")+1 : -1].strip()
                indent = '    ' * indent_level
                processed_lines.append(f"{indent}if {condition}:")
                processed_lines.append(f"{indent}    {action}")
                continue

            # 5. Obsługa bloków wieloliniowych Try/Except i standardowych pętli/if
            if raw_line == "try [":
                processed_lines.append("    " * indent_level + "try:")
                indent_level += 1
                continue
            elif raw_line.startswith('] except'):
                indent_level -= 1
                parts = raw_line[1:].strip().replace('[', ':')
                processed_lines.append("    " * indent_level + parts)
                indent_level += 1
                continue
            elif raw_line.startswith('] else'):
                indent_level -= 1
                processed_lines.append("    " * indent_level + "else:")
                indent_level += 1
                continue
            elif raw_line == ']':
                indent_level -= 1
                continue

            current_indent = "    " * max(0, indent_level)
            if raw_line.endswith('['):
                processed_lines.append(current_indent + raw_line[:-1].rstrip() + ":")
                indent_level += 1
            else:
                processed_lines.append(current_indent + raw_line)

        return "\n".join(dict.fromkeys(header_imports)) + "\n\n" + "\n".join(processed_lines)

    def generate_build_rs(self):
        linker_instructions = []
        for lib_path in self.external_libs:
            dir_name = os.path.dirname(lib_path).replace("\\", "/")
            lib_name = os.path.basename(lib_path).replace("lib", "").replace(".a", "")
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
            path = os.path.join(self.venv_dir, "Lib", "site-packages")
        else:
            path = os.path.join(self.venv_dir, "lib", f"python{sys.version_info.major}.{sys.version_info.minor}", "site-packages")
        return path.replace("\\", "\\\\")

    def _get_rust_bin_template(self):
        sp_path = self._get_site_packages_path()
        return f"""
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::ffi::c_str;
use std::env;

fn main() -> PyResult<()> {{
    Python::with_gil(|py| {{
        let sys = py.import("sys")?;
        let site_packages = "{sp_path}";

        let binding = sys.getattr("path")?;
        let path: &Bound<'_, PyList> = binding.downcast()?;
        path.insert(0, site_packages)?;

        let args: Vec<String> = env::args().collect();
        sys.setattr("argv", PyList::new(py, &args)?)?;

        let code = c_str!(include_str!("../logic.py"));

        if let Err(e) = py.run(code, None, None) {{
             eprintln!("--- HCS RUNTIME FATAL ERROR ---");
             e.print(py);
        }}
        Ok(())
    }})
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
        print(f"[*] Kompilacja start: {input_hcs}")
        self.setup_workspace()
        py_code = self.translate_hcs_to_python(input_hcs)
        with open(os.path.join(self.build_dir, "logic.py"), "w", encoding='utf-8') as f: f.write(py_code)
        self.generate_build_rs()
        self.prepare_cargo_project(is_lib)

        with open(os.path.join(self.src_dir, "main.rs"), "w") as f: f.write(self._get_rust_bin_template())
        self.build_cargo()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Użycie: python main.py <plik.hcs> [--lib]")
        sys.exit(0)
    HackerCompiler().run(sys.argv[1], "--lib" in sys.argv)
