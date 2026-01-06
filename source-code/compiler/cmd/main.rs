@ This is the self-hosted HackerScript Compiler in .hcs
@ It transpiles .hcs to .c, then compiles to ELF
@ Only manual memory supported for now

--- manual ---  @ Enable friendly manual memory (Odin-like: alloc/free with checks)

import <std:io>
import <std:fs>
import <std:process>
import <regex:parser>  @ For syntax parsing

class Compiler [
  func main(args) [
    @ Parse CLI args (simplified, like clap in Rust)
    if args.length < 2 [
      log"Usage: compile input.hcs --output=output --manual"
      return 1
    ]

    let input = args[1]
    let output = "output"  @ Default
    let manual_memory = true  @ Always for now

    if !manual_memory [
      log"Warning: Only manual supported. Forcing manual."
    ]

    @ Load Virus.hcl
    let config = load_virus_config("Virus.hcl")

    @ Transpile .hcs to .c
    let c_code = transpile_to_c(input, config)

    @ Write .c file
    fs.write(output + ".c", c_code)

    @ Compile to ELF
    compile_c(output + ".c", output, config)

    @ Cleanup
    fs.remove(output + ".c")

    log"Compiled to: " + output
    return 0
  ]

  func load_virus_config(path) [
    @ Manual memory: alloc buffer
    let buffer = alloc(1024)  @ Odin-like friendly alloc
    let data = fs.read(path, buffer)
    let config = hcl.parse(data)  @ Parse HCL
    free(buffer)  @ Manual free
    return config
  ]

  func transpile_to_c(input, config) [
    let c_code = "#include <stdio.h>\n#include <stdlib.h>\n"

    @ Add deps
    for dep in config.dependencies [
      c_code += "#include <" + dep + ">\n"
    ]

    @ Read input line by line
    let lines = fs.read_lines(input)

    for line in lines [
      @ Skip comments
      if line.starts_with("@") [ continue ]

      @ Memory markers (only manual)
      if line == "--- manual ---" [
        c_code += "// Manual memory enabled\n"
        continue
      ]
      if line.matches("--- (automatic|auto) ---") [
        c_code += "// Auto not supported; manual forced\n"
        continue
      ]

      @ Transpile import <lib:detail> -> #include <lib/detail.h>
      line = regex.replace(line, "import <(\\w+):(\\w+)>", "#include <$1/$2.h>")

      @ Transpile log"msg" -> printf("msg\n");
      line = regex.replace(line, "log\"([^\"]*)\"", "printf(\"$1\\n\");")

      @ Transpile func -> void (static type)
      line = regex.replace(line, "func (\\w+)\\(", "void $1(")

      @ Transpile class -> struct
      line = regex.replace(line, "class (\\w+)", "struct $1")

      @ Blocks: [ -> { , ] -> }
      line = line.replace("[", "{")
      line = line.replace("]", "}")

      @ Manual memory: replace new with malloc
      if line.contains("new") [
        line = line.replace("new", "malloc(sizeof")
        @ Assume user handles free
      ]

      c_code += line + "\n"
    ]

    @ Add main if needed
    if !c_code.contains("int main(") [
      c_code += "int main() { return 0; }\n"
    ]

    return c_code
  ]

  func compile_c(c_file, output, config) [
    let cmd = "gcc -o " + output + " " + c_file + " -static"
    for dep in config.dependencies [
      cmd += " -l" + dep
    ]
    cmd += " -L/home/HackerScript/libs/ -L/home/HackerScript/core/"

    process.run(cmd)
  ]
]

@ Entry point
Compiler.main(args)

