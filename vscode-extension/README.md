# 玄语 VS Code 插件

玄语编程语言的 Visual Studio Code 插件，提供语法高亮、代码补全、错误提示等功能。

## 功能特性

### 🎨 语法高亮

- 支持中文关键字高亮
- 支持英文关键字别名
- 字符串、数字、注释高亮
- 函数名、类型名高亮

### 💡 代码补全

- 关键字补全（中英文）
- 内置类型补全
- 内置函数补全
- 当前文件符号补全

### ⚠️ 错误诊断

- 实时语法检查
- 括号匹配检查
- 关键字使用检查
- 编译器错误提示

### 🔧 命令支持

| 命令 | 快捷键 | 描述 |
|------|--------|------|
| `玄语: 编译当前文件` | - | 编译当前 .xy 文件 |
| `玄语: 运行当前文件` | - | 编译并运行当前文件 |
| `玄语: 显示 LLVM IR` | - | 显示生成的 LLVM IR |

## 安装

### 从 VSIX 文件安装

1. 下载 `.vsix` 文件
2. 打开 VS Code
3. 按 `Ctrl+Shift+P` 打开命令面板
4. 输入 `Extensions: Install from VSIX`
5. 选择下载的 `.vsix` 文件

### 从源码安装

```bash
cd vscode-extension
npm install
npm run compile
# 按 F5 启动调试模式
```

## 配置

在 `settings.json` 中配置：

```json
{
  "xuanyu.compilerPath": "xy",
  "xuanyu.enableDiagnostics": true,
  "xuanyu.enableCompletion": true
}
```

| 配置项 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `xuanyu.compilerPath` | string | `"xy"` | 玄语编译器路径 |
| `xuanyu.enableDiagnostics` | boolean | `true` | 启用实时错误诊断 |
| `xuanyu.enableCompletion` | boolean | `true` | 启用代码补全 |

## 使用示例

创建一个 `hello.xy` 文件：

```xy
/**
 * 玄语示例程序
 */
函数 主(): 整数 {
    打印("你好，玄语！")
    
    定义 可变 计数: 整数 = 0
    循环 计数 < 5 {
        打印整数(计数)
        计数 = 计数 + 1
    }
    
    返回 0
}
```

按 `Ctrl+Shift+P`，输入 `玄语: 运行当前文件` 即可运行。

## 支持的关键字

### 中文关键字

| 关键字 | 英文别名 | 描述 |
|--------|----------|------|
| 函数 | fn | 定义函数 |
| 过程 | proc | 定义过程 |
| 返回 | return | 返回语句 |
| 若 | if | 条件判断 |
| 否则 | else | 否则分支 |
| 否则若 | elif | 否则若分支 |
| 循环 | loop | 循环 |
| 当 | while | 条件循环 |
| 直到 | until | 直到循环 |
| 定义 | let | 变量定义 |
| 可变 | mut | 可变变量 |
| 常量 | const | 常量定义 |
| 退出 | break | 跳出循环 |
| 跳过 | continue | 跳过本次循环 |
| 引入 | import / use | 引入模块 |
| 真 | true | 布尔真值 |
| 假 | false | 布尔假值 |

## 版本历史

### 0.1.0

- 初始版本
- 语法高亮
- 代码补全
- 错误诊断
- 编译/运行命令

## 许可证

MIT License
