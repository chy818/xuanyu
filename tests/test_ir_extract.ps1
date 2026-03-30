# test_ir_extract.ps1 - 测试 IR 提取功能

$PROJECT_ROOT = Split-Path -Parent $PSScriptRoot
$XY_COMPILER = Join-Path $PROJECT_ROOT "target\debug\xy.exe"
$MODULE_PATH = Join-Path $PROJECT_ROOT "src\compiler_v2\runtime.xy"

Write-Host "Testing IR extraction..."
Write-Host "Compiler: $XY_COMPILER"
Write-Host "Module: $MODULE_PATH"
Write-Host ""

# 运行编译器并保存输出
$output = & $XY_COMPILER $MODULE_PATH "--ir" 2>&1
Write-Host "Compiler exited with code: $LASTEXITCODE"
Write-Host ""

# 调试输出
Write-Host "=== Output lines containing markers ==="
$lines = $output -split "`n"
foreach ($line in $lines) {
    if ($line -match "LLVM" -or $line -match "成功" -or $line -match "缂") {
        Write-Host "Line: [$line]"
    }
}

Write-Host ""
Write-Host "=== Now trying to extract IR ==="

# 提取 IR
$irStart = $false
$irContent = @()

foreach ($line in $lines) {
    if ($line -match "> --- LLVM IR ---") {
        Write-Host "Found IR start marker!"
        $irStart = $true
        continue
    }
    if ($line -match "缂栬瘧鎴愬姛!") {
        Write-Host "Found success marker (garbled)!"
        break
    }
    if ($line -match "编译成功!") {
        Write-Host "Found success marker!"
        break
    }
    if ($irStart) {
        $irContent += $line
    }
}

Write-Host ""
Write-Host "Extracted IR length: $($irContent.Count) lines"
if ($irContent.Count -gt 0) {
    Write-Host "First 5 lines of IR:"
    $irContent[0..4] | ForEach-Object { Write-Host "  $_" }
    
    $irFile = Join-Path $PROJECT_ROOT "test_extracted.ll"
    $irContent -join "`n" | Out-File -FilePath $irFile -Encoding utf8
    Write-Host ""
    Write-Host "IR saved to: $irFile"
} else {
    Write-Host "ERROR: No IR extracted!"
    Write-Host ""
    Write-Host "Full output (first 50 lines):"
    $lines[0..49] | ForEach-Object { Write-Host $_ }
}
