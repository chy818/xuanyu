## LLVM 编译时如何处理中文字符

根据代码分析，项目采用以下策略处理中文：

### 1. 字符串常量（源文本中的中文字符串）

```
@str_14 = private constant [5 x i8] 
c"Pass\n"
@str_5 = private constant [1 x i8] 
c"\n"
```

- 原理 ： c"..." 语法允许任意字节序列
- UTF-8 编码的中文字符被当作普通字节流
- 字符串内容经过转义处理（ \ → \\ ， " → \" ）

### 2. 函数名

内置函数 → 映射到 ASCII 名称：

XY 函数 LLVM IR 函数 打印 print 打印整数 print_int 整数转文本 int_to_str

用户函数 → 保留中文，用引号包裹：

XY 函数 LLVM IR 函数 主 "主"

关键代码 ( codegen.rs:1587 )：

```
"主" => "主".to_string(),  // 用户函
数保留中文
```

### 3. 变量名

变量名也用引号包裹：

```
%"\u5206\u6570" = alloca i64    ; 对
应 XY 的 "分数"
```

关键代码 ( codegen.rs:107-117 )：

```
fn escape_llvm_ident(&self, name: &
str) -> String {
    if name.chars().all(|c| c.
    is_ascii_alphanumeric() || c == 
    '_') {
        name.to_string()
    } else {
        let escaped = name.replace
        ('\\', "\\\\").replace('"', 
        "\\\"");
        format!("\"{}\"", escaped)
    }
}
```

### 4. 核心映射机制

```
// 翻译函数名 - 内置函数映射到 ASCII，用
户函数保留中文
fn translate_func_name(&self, 
name: &str) -> String {
    match name {
        "打印" => Some("print"),
        "主" => Some("主"),  // 保留
        中文
        _ => None,           // 其他
        用户函数用 escape_llvm_ident 
        处理
    }
}
```

### 总结

元素 处理方式 示例 字符串内容 UTF-8 字节流 c"你好" 内置函数名 映射到 ASCII 打印 → print 用户函数名 引号包裹保留 主 → "主" 变量名 引号包裹保留 分数 → "分数"

核心思路 ：LLVM IR 的 c"..." 和 "..." 语法天然支持 UTF-8，只需要正确转义特殊字符即可。
