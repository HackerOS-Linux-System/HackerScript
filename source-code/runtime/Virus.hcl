package {
  name = "hs2"
  version = "0.1.0"
  description = "JIT Runtime for Hacker Script"
}

dependencies {
  c = {
    std = "parser"  // Biblioteka C++ do parsowania
    jit = "libjit"  // Przykładowa lib do JIT, np. LLVM lub custom
  }
  virus = {
    core = "runtime"  // Własna biblioteka HS dla runtime
  }
}

build {
  target = "hs2"
  mode = "jit"
}
