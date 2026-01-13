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

        self.home_dir = os.path.expanduser("~")
        self.pip_requirements = {"stackprinter", "numpy"}

        self.config = self._load_config(config_path)
        project_list = self.config.get('project', [{}])
        project_cfg = project_list[0] if isinstance(project_list, list) else project_list
        self.project_name = project_cfg.get('name', 'hacker_payload')

        self.rust_crates = {}
        self.manual_mode = False

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
            print(f"[*] Tworzenie izolowanego środowiska Python...")
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
            "import subprocess, sys, os, shutil, ctypes",
            "import numpy as np",
            "import stackprinter", "stackprinter.set_excepthook(style='darkbg2')"
        ]

        indent_level = 0
        in_sh_block = False
        in_block_comment = False

        for line in lines:
            raw_line = line.strip()

            # --- OBSŁUGA KOMENTARZY BLOKOWYCH ---
            if "-/" in raw_line:
                in_block_comment = True
                continue
            if "-\\" in raw_line:
                in_block_comment = False
                continue
            if in_block_comment:
                continue

            # Usuwanie komentarzy liniowych @
            raw_line = re.sub(r'@.*', '', raw_line).strip()
            if not raw_line and not in_sh_block: continue

            # --- IMPORTY SPECJALNE (RUST i C) ---
            rust_match = re.search(r'<rust:([\w\-]+)(?:=([\d\.]+))?>', raw_line)
            if rust_match:
                crate_name = rust_match.group(1)
                crate_ver = rust_match.group(2) if rust_match.group(2) else "*"
                self.rust_crates[crate_name] = crate_ver
                continue

            c_match = re.search(r'<c:(.*)>', raw_line)
            if c_match:
                lib_path = c_match.group(1)
                lib_var = os.path.basename(lib_path).split('.')[0].replace('.', '_')
                processed_lines.append(f"{'    '*indent_level}{lib_var} = ctypes.CDLL('{lib_path}')")
                continue

            # --- ZARZĄDZANIE PAMIĘCIĄ (FIXED) ---
            if "--- manual ---" in raw_line:
                self.manual_mode = True
                # Inicjalizacja libc w zależności od systemu operacyjnego
                processed_lines.append(f"{'    '*indent_level}# System Memory Management Initialization")
                if os.name == 'nt':
                    processed_lines.append(f"{'    '*indent_level}libc = ctypes.cdll.msvcrt")
                else:
                    # Na Linuxie szukamy standardowej biblioteki C
                    processed_lines.append(f"{'    '*indent_level}libc = ctypes.CDLL('libc.so.6')")
                continue

            # --- SKŁADNIA NUMPY ---
            if raw_line.startswith("matrix "):
                raw_line = raw_line.replace("matrix ", "", 1)
                if "=" in raw_line:
                    var, val = raw_line.split("=", 1)
                    raw_line = f"{var.strip()} = {val.strip()}"

            if raw_line.startswith("vector "):
                raw_line = raw_line.replace("vector ", "", 1)
                if "=" in raw_line:
                    var, val = raw_line.split("=", 1)
                    raw_line = f"{var.strip()} = np.array({val.strip()})"

            raw_line = re.sub(r'zeros\((.*)\)', r'np.zeros((\1))', raw_line)
            raw_line = re.sub(r'ones\((.*)\)', r'np.ones((\1))', raw_line)
            if " dot " in raw_line: raw_line = raw_line.replace(" dot ", " @ ")

            # --- OBSŁUGA KOMEND SH ---
            if raw_line.startswith("sh [") and raw_line.endswith("]") and not in_sh_block:
                content = re.sub(r'val_(\w+)', r'{\1}', raw_line[4:-1].strip())
                processed_lines.append(f"{'    ' * indent_level}subprocess.run(f\"\"\"{content}\"\"\", shell=True, check=True)")
                continue

            if raw_line == "sh [":
                in_sh_block = True
                continue
            if in_sh_block:
                if raw_line == "]":
                    in_sh_block = False
                    continue
                content = re.sub(r'val_(\w+)', r'{\1}', raw_line)
                processed_lines.append(f"{'    ' * indent_level}subprocess.run(f\"\"\"{content}\"\"\", shell=True, check=True)")
                continue

            # --- BLOKI I WCIĘCIA (NAPRAWA ELSE/EXCEPT) ---
            if raw_line.startswith('] except'):
                indent_level = max(0, indent_level - 1)
                line_content = raw_line[1:].strip().replace('[', ':')
                processed_lines.append(f"{'    ' * indent_level}{line_content}")
                indent_level += 1
                continue

            if raw_line.startswith('] else'):
                indent_level = max(0, indent_level - 1)
                processed_lines.append(f"{'    ' * indent_level}else:")
                indent_level += 1
                continue

            if raw_line == ']':
                indent_level = max(0, indent_level - 1)
                continue

            # Tłumaczenie słów kluczowych
            raw_line = re.sub(r'(import\s+)?<core:([\w\.]+)>', r'import \2', raw_line)
            if raw_line.startswith('func '): raw_line = raw_line.replace('func ', 'def ', 1)
            if raw_line.startswith('log '): raw_line = f"print({raw_line[4:].strip()})"

            # Otwieranie bloków
            if raw_line.endswith('['):
                processed_lines.append(f"{'    ' * indent_level}{raw_line[:-1].strip()}:")
                indent_level += 1
            else:
                processed_lines.append(f"{'    ' * indent_level}{raw_line}")

        return "\n".join(dict.fromkeys(header_imports)) + "\n\n" + "\n".join(processed_lines)

    def prepare_cargo_project(self, is_lib):
        lib_section = f'[lib]\nname = "{self.project_name}"\ncrate-type = ["staticlib"]' if is_lib else ""
        rust_deps = ""
        for name, ver in self.rust_crates.items():
            rust_deps += f'{name} = "{ver}"\n'

        cargo_toml = f"""
[package]
name = "{self.project_name}"
version = "0.1.0"
edition = "2021"

{lib_section}

[dependencies]
pyo3 = {{ version = "0.23.5", features = ["auto-initialize"] }}
{rust_deps}

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
"""
        with open(os.path.join(self.build_dir, "Cargo.toml"), "w") as f: f.write(cargo_toml)

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

    def _get_site_packages_path(self):
        if os.name == "nt":
            return os.path.join(self.venv_dir, "Lib", "site-packages").replace("\\", "\\\\")
        return os.path.join(self.venv_dir, "lib", f"python{sys.version_info.major}.{sys.version_info.minor}", "site-packages")

    def build_cargo(self):
        python_exe = os.path.join(self.venv_dir, "Scripts", "python.exe") if os.name == "nt" else os.path.join(self.venv_dir, "bin", "python")
        os.environ["PYTHON_SYS_EXECUTABLE"] = python_exe
        os.environ["RUSTFLAGS"] = "-C target-cpu=native"

        res = subprocess.run(["cargo", "build", "--release"], cwd=self.build_dir)
        if res.returncode == 0:
            print(f"[+] Sukces! Binarka gotowa w {os.path.join(self.build_dir, 'target/release/')}")
        else:
            sys.exit(1)

    def run(self, input_hcs, is_lib=False):
        # 1. Analiza i translacja
        py_code = self.translate_hcs_to_python(input_hcs)

        # 2. Workspace
        self.setup_workspace()

        # 3. Zapis plików projektu
        with open(os.path.join(self.build_dir, "logic.py"), "w", encoding='utf-8') as f: f.write(py_code)
        self.prepare_cargo_project(is_lib)
        os.makedirs(self.src_dir, exist_ok=True)
        with open(os.path.join(self.src_dir, "main.rs"), "w") as f: f.write(self._get_rust_bin_template())

        # 4. Kompilacja Cargo
        self.build_cargo()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Użycie: python main.py <plik.hcs> [--lib]")
        sys.exit(0)
    HackerCompiler().run(sys.argv[1], "--lib" in sys.argv)
