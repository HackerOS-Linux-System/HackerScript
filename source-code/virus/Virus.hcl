package {
  name = "virus-cli"
  version = "0.1.0"
  description = "CLI Tool for managing Hacker Script projects, builds, and packages"
}

dependencies {
  c = {
    std = "io"      // For input/output
    term = "ansi"   // For colored CLI output
    curl = "lib"    // For HTTP downloads
    tar = "lib"     // For unpacking tar.gz
    json = "parser" // For parsing JSON
  }
  virus = {
    core = "fs"     // File system utilities
    core = "proc"   // Process execution utilities
  }
}

build {
  target = "virus"
  mode = "cli"
  install_path = "/usr/bin/virus"
  permissions = "0755"
}
