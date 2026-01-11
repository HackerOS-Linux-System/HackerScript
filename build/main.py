import re
import sys
import os
import subprocess
import hcl2

class HackerCompiler:
    def __init__(self, config_path="Virus.hcl"):
        self.cache_dir = "cache"
        self.env_dir = os.path.join(self.cache_dir, "env")
        self.build_dir = os.path.join(self.cache_dir, "build")
        self.src_dir = os.path.join(self.build_dir, "src")

        # Ładowanie konfiguracji HCL
        self.config = self._load_config(config_path)

        # Bezpieczne pobieranie danych z listy HCL
        project_list = self.config.get('project', [{}])
        project_cfg = project_list[0] if isinstance(project_list, list) else project_list
        self.project_name = project_cfg.get('name', 'virus_payload')

        build_list = self.config.get('build', [{}])
        build_cfg = build_list[0] if isinstance(build_list, list) else build_list
        self.binary_name = build_cfg.get('binary_name', 'payload')

    def _load_config(self, path):
        if not os.path.exists(path):
            print(f"[-] OSTRZEŻENIE: Brak pliku {path}. Używam domyślnych ustawień.")
            return {}
        try:
            with open(path, 'r', encoding='utf-8') as f:
                return hcl2.load(f)
        except Exception as e:
            print(f"[-] BŁĄD PARSOWANIA HCL: {e}")
            return {}

    def setup_workspace(self):
        """Tworzy strukturę folderów cache"""
        for directory in [self.env_dir, self.src_dir]:
            if not os.path.exists(directory):
                os.makedirs(directory, exist_ok=True)
        print(f"[+] Workspace przygotowany w /{self.cache_dir}")

    def translate_hcs_to_python(self, hcs_path):
        """Konwertuje .hcs na czysty Python"""
        if not os.path.exists(hcs_path):
            raise FileNotFoundError(f"Plik źródłowy {hcs_path} nie istnieje.")

        with open(hcs_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        processed_lines = []
        indent_level = 0

        for line in lines:
            raw_line = line.strip()
            raw_line = re.sub(r'@.*', '', raw_line).strip() # Komentarze

            if not raw_line:
                processed_lines.append("")
                continue

            # Importy <core:nazwa>
            raw_line = re.sub(r'(import\s+)?<core:([\w\.]+)>', r'import \2', raw_line)

            # Słowa kluczowe
            raw_line = raw_line.replace('--- automatic ---', '').replace('--- manual ---', '')

            if raw_line.startswith('func '):
                raw_line = raw_line.replace('func ', 'def ', 1)

            if raw_line.startswith('log '):
                content = raw_line[4:].strip()
                raw_line = f"print({content})"

            # Obsługa bloków [ ]
            if raw_line == ']':
                indent_level -= 1
                continue

            current_indent = "    " * max(0, indent_level)

            if raw_line.endswith('['):
                formatted_line = current_indent + raw_line[:-1].rstrip() + ":"
                processed_lines.append(formatted_line)
                indent_level += 1
            else:
                formatted_line = current_indent + raw_line
                processed_lines.append(formatted_line)

        return "\n".join(processed_lines)

    def prepare_rust_project(self, python_code):
        """Tworzy projekt Rust z poprawnymi zależnościami PyO3"""
        py_out_path = os.path.join(self.build_dir, "logic.py")
        with open(py_out_path, "w", encoding='utf-8') as f:
            f.write(python_code)

        # Cargo.toml - Używamy sprawdzonych flag dla wersji 0.23
        cargo_content = f"""
[package]
name = "{self.project_name}"
version = "0.1.0"
edition = "2021"

[dependencies]
pyo3 = {{ version = "0.23.3", features = ["auto-initialize"] }}
"""
        with open(os.path.join(self.build_dir, "Cargo.toml"), "w") as f:
            f.write(cargo_content)

        # src/main.rs - Nowoczesne API Bound
        rust_content = r"""
use pyo3::prelude::*;

fn main() -> PyResult<()> {
    // Automatyczna inicjalizacja interpretera dzięki fladze auto-initialize
    Python::with_gil(|py| {
        let code = include_str!("../logic.py");
        let res = py.run_bound(code, None, None);

        if let Err(e) = res {
            eprintln!("Python Script Error: {:?}", e);
        }
        Ok(())
    })
}
"""
        with open(os.path.join(self.src_dir, "main.rs"), "w") as f:
            f.write(rust_content)

    def compile_binary(self):
        """Uruchamia kompilację Cargo"""
        print(f"[*] Kompilowanie projektu '{self.project_name}' do binarki...")
        try:
            # Używamy --release dla optymalizacji i mniejszego rozmiaru
            result = subprocess.run(
                ["cargo", "build", "--release"],
                cwd=self.build_dir
            )
            if result.returncode == 0:
                target_path = os.path.join(self.build_dir, "target", "release", self.project_name)
                print("-" * 40)
                print(f"[+] KOMPILACJA ZAKOŃCZONA SUKCESEM!")
                print(f"[+] Lokalizacja: {target_path}")
                print("-" * 40)
            else:
                print("[-] Błąd kompilacji Cargo. Sprawdź logi powyżej.")
        except FileNotFoundError:
            print("[-] BŁĄD: Nie znaleziono Cargo. Zainstaluj Rust.")

    def run(self, hcs_filename):
        hcs_path = hcs_filename
        if not os.path.exists(hcs_path):
            hcs_path = os.path.join("cmd", hcs_filename)

        try:
            print(f"[*] Przetwarzanie: {hcs_path}")
            self.setup_workspace()

            py_code = self.translate_hcs_to_python(hcs_path)
            self.prepare_rust_project(py_code)
            self.compile_binary()

        except Exception as e:
            print(f"[-] KRYTYCZNY BŁĄD: {e}")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        compiler = HackerCompiler()
        compiler.run(sys.argv[1])
    else:
        print("Sposób użycia: python3 main.py <plik.hcs>")

