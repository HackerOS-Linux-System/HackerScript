use pyo3::prelude::*;
use std::env;

fn main() -> PyResult<()> {
    // Pobieramy argumenty przekazane do binarki (np. "main.hcs")
    let args: Vec<String> = env::args().collect();

    Python::with_gil(|py| {
        // Ustawiamy sys.argv wewnątrz interpretera Pythona, 
        // aby Twój kod Python "widział" argumenty binarki.
        let sys = py.import_bound("sys")?;
        sys.setattr("argv", args)?;

        let code = include_str!("../main.py");
        let res = py.run_bound(code, None, None);

        if let Err(e) = res {
            eprintln!("Python Script Error:");
            e.print(py);
        }
        Ok(())
    })
}
