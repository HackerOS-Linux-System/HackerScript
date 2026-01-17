# src/hcs.cr
# hcs - HackerScript Compiler & Toolchain CLI
# Umieszczane w /usr/bin/hcs

require "option_parser"
require "file_utils"
require "http/client"
require "json"

VERSION = "0.1.0-dev"

HOME_DIR       = ENV["HOME"]? || "~"
HS_HOME        = "#{HOME_DIR}/.HackerScript"
BIN_DIR        = "#{HS_HOME}/bin"
LIBS_DIR       = "#{HS_HOME}/libs"
VIRA_INDEX_URL = "https://raw.githubusercontent.com/HackerOS-Linux-System/HackerScript/main/repo/vira-index.json"
VIRUS_INDEX_URL = "https://raw.githubusercontent.com/HackerOS-Linux-System/HackerScript/main/repo/index.json"

enum BuildMode
  Bytecode
  Native
end

def main
  command = ""
  input_file = ""
  output_file : String? = nil
  mode = BuildMode::Bytecode
  verbose = false
  show_help = false

  parser = OptionParser.new do |opts|
    opts.banner = "Użycie: hcs [polecenie] [opcje] [plik]"

    opts.on("-h", "--help", "Pokazuje tę pomoc") do
      show_help = true
    end

    opts.on("-v", "--version", "Pokazuje wersję") do
      puts "hcs #{VERSION}"
      exit
    end

    opts.on("-V", "--verbose", "Więcej informacji podczas działania") do
      verbose = true
    end

    opts.separator ""

    opts.on("compile", "Kompiluje plik .hcs do bytecode (.bc) lub natywnego kodu") do
      command = "compile"
      opts.on("-o FILE", "--output=FILE", "Plik wyjściowy") { |f| output_file = f }
      opts.on("--native", "Kompiluj do natywnego kodu (tryb manual)") { mode = BuildMode::Native }
    end

    opts.on("run", "Kompiluje i natychmiast uruchamia") do
      command = "run"
      opts.on("--native", "Uruchom w trybie natywnego kodu") { mode = BuildMode::Native }
    end

    opts.on("check", "Sprawdza składnię bez generowania kodu") do
      command = "check"
    end

    opts.on("hspm", "Zarządzanie pakietami (przekierowanie do hspm)") do
      command = "hspm"
    end

    opts.on("virus", "Zarządzanie prywatnym repozytorium (przekierowanie do virus)") do
      command = "virus"
    end

    opts.invalid_option do |flag|
      STDERR.puts "Nieznana opcja: #{flag}"
      STDERR.puts opts
      exit 1
    end

    opts.missing_option do |flag|
      STDERR.puts "Brakująca wartość dla opcji: #{flag}"
      exit 1
    end
  end

  begin
    parser.parse
  rescue ex : OptionParser::InvalidOption | OptionParser::MissingOption
    STDERR.puts ex.message
    STDERR.puts parser
    exit 1
  end

  if show_help || command.empty?
    puts parser
    puts "\nDostępne polecenia:"
    puts "  compile    Kompiluj plik źródłowy"
    puts "  run        Kompiluj i uruchom"
    puts "  check      Sprawdź składnię"
    puts "  hspm       Zarządzanie pakietami (alias)"
    puts "  virus      Zarządzanie prywatnym repo (alias)"
    exit 0
  end

  # Pozostałe argumenty (po opcji) → plik wejściowy + args do programu
  args = ARGV.dup
  if !args.empty?
    input_file = args.shift
  end

  case command
  when "compile"
    compile(input_file, output_file, mode, verbose)

  when "run"
    compile_and_run(input_file, mode, verbose, args)

  when "check"
    check_syntax(input_file, verbose)

  when "hspm"
    exec_hspm(args)

  when "virus"
    exec_virus(args)

  else
    STDERR.puts "Nieznane polecenie: #{command}"
    exit 1
  end
end

def compile(input_file : String, output_file : String?, mode : BuildMode, verbose : Bool)
  unless File.file?(input_file)
    STDERR.puts "Plik nie istnieje: #{input_file}"
    exit 1
  end

  unless input_file.ends_with?(".hcs")
    STDERR.puts "Oczekiwano pliku .hcs"
    exit 1
  end

  out_file = output_file || input_file.sub(/\.hcs$/, mode == BuildMode::Native ? ".bin" : ".bc")

  puts "Kompilacja #{input_file} → #{out_file} (tryb: #{mode})..." if verbose

  # Tutaj uruchamiamy właściwy kompilator HS1
  hs1_path = "#{BIN_DIR}/hs1"
  unless File.executable?(hs1_path)
    STDERR.puts "Nie znaleziono HS1 w #{hs1_path}"
    exit 1
  end

  cmd = [hs1_path, "compile", "--input", input_file]
  cmd << "--output" << out_file if output_file
  cmd << "--native" if mode == BuildMode::Native

  status = Process.run(cmd, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  exit status.exit_code unless status.success?
end

def compile_and_run(input_file : String, mode : BuildMode, verbose : Bool, extra_args : Array(String))
  temp_bc = "/tmp/hcs-run-#{Random::Secure.hex(8)}.bc"
  compile(input_file, temp_bc, mode, verbose)

  runner = mode == BuildMode::Native ? temp_bc : "#{BIN_DIR}/hs2"

  cmd = [runner]
  cmd << temp_bc if mode == BuildMode::Bytecode
  cmd.concat(extra_args)

  puts "Uruchamianie: #{cmd.join(" ")}" if verbose

  status = Process.run(cmd, output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  File.delete?(temp_bc)
  exit status.exit_code
end

def check_syntax(input_file : String, verbose : Bool)
  hs3_path = "#{BIN_DIR}/hs3"
  unless File.executable?(hs3_path)
    STDERR.puts "Nie znaleziono HS3 (lexer/parser)"
    exit 1
  end

  status = Process.run([hs3_path, input_file], output: Process::Redirect::Inherit, error: Process::Redirect::Inherit)
  if status.success?
    puts "Składnia OK: #{input_file}"
  else
    exit 1
  end
end

def exec_hspm(args : Array(String))
  hspm_path = "#{BIN_DIR}/hspm"  # zakładamy że to skrypt hcs lub binarka
  full_cmd = [hspm_path] + args
  Process.exec(full_cmd.first, full_cmd[1..-1])
end

def exec_virus(args : Array(String))
  virus_path = "/usr/bin/virus"
  unless File.executable?(virus_path)
    STDERR.puts "Nie znaleziono /usr/bin/virus"
    exit 1
  end
  Process.exec(virus_path, args)
end

main
