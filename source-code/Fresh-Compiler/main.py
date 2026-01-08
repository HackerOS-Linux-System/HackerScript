import lark
import lark.visitors
import os
import subprocess
import sys
sys.setrecursionlimit(20000)
# Define the Lark grammar for the new HackerScript syntax
grammar = r"""
%import common.WS
%ignore WS
%import common.NEWLINE
%ignore NEWLINE
%import common.CNAME -> ID
%import common.INT -> INT
%import common.ESCAPED_STRING -> STRING
COMMENT: /@.*/
%ignore COMMENT
directive: "---" MODE "---"
MODE: "manual" | "automatic"
?import_inner: colon_inner | nested_inner
colon_inner: ":" ID
nested_inner: "<" ID ":" ID ">"
import_stmt: "import" "<" ID import_inner ">"
program: directive? import_stmt* class_def* func_def*
start: program
class_def: "class" ID "[" func_def* "]"
func_def: "func" ID "(" params? ")" "[" statement* "]"
params: ID ("," ID)*
args: expr ("," expr)*
?statement: assignment
  | log_stmt
  | func_call_stmt
  | return_stmt
  | if_stmt
  | for_stmt
assignment: expr "=" expr
log_stmt: "log" STRING
func_call_stmt: func_call
func_call: expr "(" args? ")"
return_stmt: "return" expr?
if_stmt: "if" expr "[" statement* "]" else_if_part* else_block_part?
else_if_part: "else" "if" expr "[" statement* "]"
else_block_part: "else" "[" statement* "]"
for_stmt: "for" ID "in" expr "[" statement* "]"
?expr: logic
?logic: compare_term ("&&" compare_term)*
?compare_term: compare | "!" add -> not_expr
?compare: add (("=="|"<"|">") add)*
?add: value ("+" value)*
?value: atom
  | value "." ID -> dot_access
  | value "(" args? ")" -> call
  | value "[" expr "]" -> array_access
atom: INT -> int_literal
    | STRING -> string_literal
    | "null" -> null_literal
    | ID -> var_access
    | "[" args? "]" -> array_literal
    | "new" ID "(" ")" -> new_expr
    | "(" expr ")" -> paren_expr
"""
def get_value(node):
    if isinstance(node, lark.Tree):
        if node.data in ['int_literal', 'string_literal', 'var_access', 'new_expr']:
            return get_value(node.children[0])
        else:
            return str(node)
    elif isinstance(node, lark.Token):
        return node.value
    else:
        return str(node)
# Collector for fields and class names
class Collector(lark.visitors.Interpreter):
    def __init__(self):
        self.current_class = None
        self.class_fields = {}
        self.class_names = set()
    def class_def(self, tree):
        name = get_value(tree.children[1])
        self.class_names.add(name)
        self.current_class = name
        if name not in self.class_fields:
            self.class_fields[name] = {}
        self.visit_children(tree)
        self.current_class = None
    def assignment(self, tree):
        if len(tree.children) < 3:
            return
        left = tree.children[0]
        right = tree.children[2]
        if not isinstance(left, lark.Tree) or left.data != 'dot_access':
            return
        if len(left.children) < 3:
            return
        left_left = left.children[0]
        if not isinstance(left_left, lark.Tree) or left_left.data != 'var_access':
            return
        if len(left_left.children) < 1:
            return
        if get_value(left_left.children[0]) != 'self' or not self.current_class:
            return
        field_name = get_value(left.children[2])
        field_type = self.infer_type(right)
        self.class_fields[self.current_class][field_name] = field_type
    def infer_type(self, tree):
        if not isinstance(tree, lark.Tree):
            return "unknown"
        if tree.data == 'string_literal':
            return "char*"
        elif tree.data == 'int_literal':
            return "int"
        elif tree.data == 'null_literal':
            return "void*"
        elif tree.data == 'add':
            left_t = self.infer_type(tree.children[0])
            for i in range(1, len(tree.children), 2):
                right_t = self.infer_type(tree.children[i+1])
                if left_t == "char*" or right_t == "char*":
                    left_t = "char*"
                else:
                    left_t = "int"
            return left_t
        elif tree.data == 'call':
            func = tree.children[0]
            if isinstance(func, lark.Tree) and func.data == 'var_access':
                name = get_value(func.children[0])
                if name == 'allocate':
                    return "void*"
                elif name in ['read_file', 'get_current_version', 'get_remote_version', 'replace', 'get_cwd']:
                    return "char*"
                elif name == 'curl_get':
                    return "Response*"
                elif name == 'json_parse':
                    return "Json*"
                elif name == 'list_dir':
                    return "Array"
                elif name == 'parse_hcl':
                    return "Hcl*"
            return "unknown"
        elif tree.data == 'new_expr':
            name = get_value(tree.children[1])
            return f"struct {name}*"
        elif tree.data == 'var_access':
            return "unknown"
        else:
            return "unknown"
# Transformer to generate C code
class CTransformer(lark.Transformer):
    def __init__(self, class_fields, class_names):
        self.mode = "automatic"
        self.scopes = [{}]
        self.current_class = None
        self.class_fields = class_fields
        self.class_names = class_names
        self.has_main = False
        self.current_ret_type = None
    def process_interpolation(self, s):
        s = s[1:-1] # remove quotes
        if '{' not in s:
            return f'"{s}"'
        parts = []
        vars = []
        current = ''
        i = 0
        while i < len(s):
            if s[i] == '{':
                if current:
                    parts.append(current)
                current = ''
                i += 1
                var = ''
                while i < len(s) and s[i] != '}':
                    var += s[i]
                    i += 1
                i += 1 # skip }
                vars.append(var.strip())
                parts.append("%s")
            else:
                current += s[i]
                i += 1
        if current:
            parts.append(current)
        format_str = ''.join(parts)
        args = ', '.join(vars)
        return f'(char*)({{ char *str = NULL; asprintf(&str, "{format_str}", {args}); str; }})'
    def directive(self, children):
        if len(children) > 1:
            self.mode = children[1].value
        return ""
    def import_stmt(self, children):
        if len(children) < 4:
            return ''
        category = children[2].value
        inner = children[3]
        if inner.data == 'colon_inner':
            if len(inner.children) < 2:
                return ''
            lib = inner.children[1].value
            incl = lib
        elif inner.data == 'nested_inner':
            if len(inner.children) < 4:
                return ''
            module = inner.children[1].value
            part = inner.children[3].value
            incl = f"{module}/{part}"
        else:
            return ''
        if category == "c":
            return f'#include <{incl}.h>\n'
        elif category == "virus":
            return f'#include "{incl}.h"\n'
        return ''
    def params(self, children):
        return [get_value(c) for c in children if c.type == 'ID']
    def args(self, children):
        return [c for c in children if isinstance(c, tuple)]
    def var_access(self, children):
        if len(children) < 1:
            return '', 'unknown'
        name = get_value(children[0])
        type_ = self.get_type(name)
        if type_ is None:
            type_ = "unknown"
        return name, type_
    def dot_access(self, children):
        if len(children) < 3:
            return '', 'unknown'
        left_c, left_t = children[0]
        right = get_value(children[2])
        if left_t.startswith('struct ') and left_t.endswith('*'):
            code = f'{left_c}->{right}'
            class_name = left_t[7:-1]
            type_ = self.class_fields.get(class_name, {}).get(right, "char*")
        elif left_t == "Array":
            if right == "length":
                code = f'{left_c}.len'
                type_ = "int"
            else:
                code = f'{left_c}.{right}'
                type_ = "unknown"
        elif left_t == "Response*":
            code = f'{left_c}->{right}'
            if right == "status":
                type_ = "int"
            else:
                type_ = "char*"
        else:
            code = f'{left_c}->{right}'
            type_ = "unknown"
        return code, type_
    def call(self, children):
        if len(children) < 1:
            return '', 'unknown'
        func = children[0]
        args = children[2] if len(children) > 2 else []
        args_c = ', '.join(a[0] for a in args)
        func_c, func_t = func
        if func_t == "unknown":
            name = func_c
            if name == 'allocate':
                code = f'malloc({args_c})'
                type_ = "void*"
            elif name == 'deallocate':
                code = f'free({args_c})'
                type_ = "void"
            elif name == 'version_compare':
                code = f'strcmp({args_c})'
                type_ = "int"
            else:
                code = f'{name}({args_c})'
                type_ = self.infer_call_type(name)
        else:
            method = func_c.split('->')[-1] if '->' in func_c else func_c
            full_name = f'{func_t[:-1] if func_t.endswith("*") else func_t}_{method}' if func_t[:-1] in self.class_names else method
            args_c = f'{func_c}' + (', ' + args_c if args_c else '')
            code = f'{full_name}({args_c})'
            type_ = self.infer_call_type(method)
        return code, type_
    def func_call(self, children):
        return children[0]
    def infer_call_type(self, name):
        if name in ['build', 'run', 'install', 'remove', 'version_compare']:
            return "int"
        elif name in ['get_current_version', 'get_remote_version', 'replace', 'get_cwd', 'read_file', 'write_file', 'read_input']:
            return "char*"
        elif name == 'curl_get':
            return "Response*"
        elif name == 'json_parse':
            return "Json*"
        elif name == 'parse_hcl':
            return "Hcl*"
        elif name == 'list_dir':
            return "Array"
        elif name == 'file_exists':
            return "bool"
        else:
            return "unknown"
    def new_expr(self, children):
        if len(children) < 2:
            return '', 'unknown'
        name = get_value(children[1])
        code = f'(struct {name}*)malloc(sizeof(struct {name}))'
        return code, f"struct {name}*"
    def string_literal(self, children):
        if len(children) < 1:
            return '', 'char*'
        s = children[0].value
        return self.process_interpolation(s), "char*"
    def int_literal(self, children):
        if len(children) < 1:
            return '', 'int'
        return children[0].value, "int"
    def null_literal(self, children):
        return "NULL", "void*"
    def array_literal(self, children):
        args = children[1] if len(children) == 3 else []
        codes = [a[0] for a in args]
        n = len(codes)
        code = f'(Array){{.data = (char**)(char*[]){{ {", ".join(codes)} }}, .len = {n} }}'
        return code, "Array"
    def paren_expr(self, children):
        return children[1]
    def add(self, children):
        if len(children) == 1:
            return children[0]
        left = children[0]
        i = 1
        while i + 1 < len(children):
            right = children[i+1]
            left_c, left_t = left
            right_c, right_t = right
            if left_t == "char*" or right_t == "char*":
                code = f'(char*)({{ char *res = NULL; asprintf(&res, "%s%s", {left_c}, {right_c}); res; }})'
                left_t = "char*"
            else:
                code = f'({left_c} + {right_c})'
                left_t = "int"
            left = code, left_t
            i += 2
        return left
    def compare(self, children):
        if len(children) == 1:
            return children[0]
        left = children[0]
        i = 1
        while i + 1 < len(children):
            op_node = children[i]
            op = op_node.value if hasattr(op_node, 'value') else op_node.children[0].value
            right = children[i+1]
            left_c, left_t = left
            right_c, right_t = right
            if op == "==":
                if left_t == "char*" and right_t == "char*":
                    code = f'(strcmp({left_c}, {right_c}) == 0)'
                else:
                    code = f'({left_c} == {right_c})'
            elif op == "<":
                code = f'({left_c} < {right_c})'
            elif op == ">":
                code = f'({left_c} > {right_c})'
            left = code, "bool"
            i += 2
        return left
    def logic(self, children):
        if len(children) == 1:
            return children[0]
        left = children[0]
        i = 1
        while i + 1 < len(children):
            right = children[i+1]
            left_c, left_t = left
            right_c, right_t = right
            code = f'({left_c} && {right_c})'
            left = code, "bool"
            i += 2
        return left
    def not_expr(self, children):
        if len(children) < 2:
            return '', 'bool'
        expr_c, expr_t = children[1]
        code = f'!({expr_c})'
        return code, "bool"
    def array_access(self, children):
        if len(children) < 3:
            return '', 'unknown'
        left_c, left_t = children[0]
        idx_c, idx_t = children[2]
        if left_t == "Array":
            code = f'{left_c}.data[{idx_c}]'
            type_ = "char*"
        elif left_t == "Json*":
            code = f'{left_c}->items.data[{idx_c}]'
            type_ = "char*"
        else:
            code = f'{left_c}[{idx_c}]'
            type_ = "unknown"
        return code, type_
    def get_type(self, name):
        for scope in reversed(self.scopes):
            if name in scope:
                return scope[name]
        return None
    def assignment(self, children):
        if len(children) < 3:
            return ''
        left_c, left_t = children[0]
        right_c, right_t = children[2]
        if self.get_type(left_c) is None and ' ' not in left_c and '->' not in left_c and '.' not in left_c and not left_c.startswith('('):
            self.scopes[-1][left_c] = right_t if right_t != "unknown" else "char*"
        return f'{left_c} = {right_c};\n'
    def log_stmt(self, children):
        if len(children) < 2:
            return ''
        s_c = self.process_interpolation(children[1].value)
        return f'printf("%s\\n", {s_c});\n'
    def return_stmt(self, children):
        if len(children) < 1:
            return ''
        if len(children) == 1:
            return 'return;\n'
        expr_c, expr_t = children[1]
        return f'return {expr_c};\n'
    def func_call_stmt(self, children):
        if len(children) < 1:
            return ''
        call_c, call_t = children[0]
        return f'{call_c};\n'
    def else_if_part(self, children):
        if len(children) < 5:
            return ''
        cond_c, cond_t = children[2]
        block = ''.join([c for c in children[4:] if isinstance(c, str)])
        return f' else if ({cond_c}) {{ {block} }}'
    def else_block_part(self, children):
        if len(children) < 3:
            return ''
        block = ''.join([c for c in children[2:] if isinstance(c, str)])
        return f' else {{ {block} }}'
    def if_stmt(self, children):
        if len(children) < 4:
            return ''
        cond = children[1]
        if isinstance(cond, tuple):
            cond_c, cond_t = cond
        else:
            cond_c = str(cond)
            cond_t = "bool"
        # Find position of ]
        block_end = len(children)
        for i in range(3, len(children)):
            if isinstance(children[i], lark.Token) and children[i].value == ']':
                block_end = i
                break
        block = ''.join([c for c in children[3:block_end] if isinstance(c, str)])
        else_parts = ''.join([c for c in children[block_end+1:] if isinstance(c, str)])
        return f'if ({cond_c}) {{ {block} }}{else_parts}\n'
    def for_stmt(self, children):
        if len(children) < 6:
            return ''
        var = get_value(children[1])
        collection = children[3]
        if isinstance(collection, tuple):
            collection_c, collection_t = collection
        else:
            collection_c = str(collection)
            collection_t = "Array"
        block = ''.join([c for c in children[5:] if isinstance(c, str)])
        code = f'for (int _{var}_i = 0; _{var}_i < {collection_c}.len; _{var}_i++) {{ char* {var} = {collection_c}.data[_{var}_i]; {block} }}\n'
        return code
    def func_def(self, children):
        if len(children) < 5:
            return ''
        name = get_value(children[1])
        self.scopes.append({})
        if name == "main":
            self.has_main = True
            name = "hs_main"
        if self.current_class:
            name = f'{self.current_class}_{name}'
        # Find params - it's the child that has data == 'params'
        params_idx = -1
        for i, child in enumerate(children):
            if isinstance(child, lark.Tree) and child.data == 'params':
                params_idx = i
                break

        param_list = self.params(children[params_idx].children) if params_idx != -1 else []

        # Collect all statement strings that come after the opening bracket
        block_c = ''.join([c for c in children if isinstance(c, str)])

        for p in param_list:
            t = "Array" if p == "args" else "char*"
            self.scopes[-1][p] = t
        locals_decl = ''.join(f'{t if t != "unknown" else "char*"} {name_var};\n' for name_var, t in self.scopes[-1].items() if name_var not in param_list)
        block_c = locals_decl + block_c
        self.scopes.pop()
        param_str = ', '.join(f'{self.get_type(p) or "char*"} {p}' for p in param_list)
        ret_type = "int" if name in ["hs_main", "build", "run", "install", "remove", "version_compare"] else "void"
        self_param = f'struct {self.current_class}* self, ' if self.current_class else ''
        return f'{ret_type} {name}({self_param}{param_str}) {{ {block_c} }}\n'
    def class_def(self, children):
        if len(children) < 4:
            return ''
        name = get_value(children[1])
        old_class = self.current_class
        self.current_class = name
        funcs_c = ''.join([c for c in children[3:] if isinstance(c, str)])
        self.current_class = old_class
        field_str = ''.join(f'{t} {f};\n' for f, t in self.class_fields.get(name, {}).items())
        if not field_str:
            field_str = 'char dummy; // to avoid empty struct\n'
        return f'struct {name} {{ {field_str} }};\n{funcs_c}'
    def program(self, children):
        imports = ''.join(i for i in children if isinstance(i, str) and i.startswith('#include'))
        defs = ''.join(i for i in children if isinstance(i, str) and not i.startswith('#include'))
        header = '#define _GNU_SOURCE\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <stdbool.h>\n#include <unistd.h>\n#include <curl/curl.h>\n' + """typedef struct { char** data; int len; } Array;
bool array_contains(Array a, char* s) { for(int i=0; i<a.len; i++) if(strcmp(a.data[i], s)==0) return true; return false; }
Array array_slice(Array a, int start) { return (Array){a.data + start, a.len - start}; }
char* array_last(Array a) { return a.data[a.len-1]; }
typedef struct { int status; char* body; } Response;
typedef struct { Array items; } Json;
typedef struct { struct { Array c; Array virus; } dependencies; } Hcl;
"""
        if self.mode == "manual":
            header += """#define defer(stmt) for (int _i = 1; _i--; (stmt)) """
        if self.has_main:
            defs += '\nint main(int argc, char** argv) { Array args = {argv + 1, argc - 1}; return hs_main(args); }\n'
        return header + imports + defs
    def start(self, children):
        return children[0]
# Main compiler function
def compile_hcs(input_file, output_bin=None):
    with open(input_file, 'r') as f:
        text = f.read()
    parser = lark.Lark(grammar, start='start', parser='earley')
    tree = parser.parse(text)
    collector = Collector()
    collector.visit(tree)
    transformer = CTransformer(collector.class_fields, collector.class_names)
    c_code = transformer.transform(tree)
    tmp_c_file = f'/tmp/{os.path.basename(input_file)}.c'
    with open(tmp_c_file, 'w') as f:
        f.write(c_code)
    if not output_bin:
        output_bin = os.path.basename(input_file).replace('.hcs', '')
    script_dir = os.path.dirname(os.path.abspath(__file__))
    compile_cmd = [
        'gcc', tmp_c_file, '-o', output_bin,
        f'-I{os.path.join(script_dir, "core/")}',
        f'-I{os.path.join(script_dir, "libs/")}',
        f'-L{os.path.join(script_dir, "libs/")}',
        '-lcurl'
    ]
    try:
        subprocess.check_call(compile_cmd)
        print(f"Compiled {input_file} to {output_bin}")
    finally:
        if os.path.exists(tmp_c_file):
            os.remove(tmp_c_file)
if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: HackerScript-Compiler <file.hcs>")
        sys.exit(1)
    compile_hcs(sys.argv[1])

