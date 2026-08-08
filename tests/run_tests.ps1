# Xuanyu Compiler Test Runner
# Run all tests and generate report

param(
    [switch]$All,
    [switch]$Unit,
    [switch]$Integration,
    [switch]$Bootstrap,
    [switch]$Verbose
)

# Test statistics
$Total = 0
$Passed = 0
$Failed = 0
$Skipped = 0

# Project paths
$ProjectRoot = $PSScriptRoot
# If this script sits in a tests/ subdir of the repo, resolve up to the repo root
if (Test-Path (Join-Path (Split-Path $ProjectRoot -Parent) "src")) {
    $ProjectRoot = Split-Path $ProjectRoot -Parent
}
$SrcDir = Join-Path $ProjectRoot "src"
$TestsDir = Join-Path $ProjectRoot "tests"
$ExamplesDir = Join-Path $ProjectRoot "examples"
$OutputDir = Join-Path $ProjectRoot "output"

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
}

# Print functions
function Print-Success { Write-Host "[PASS] $args" -ForegroundColor Green }
function Print-Error { Write-Host "[FAIL] $args" -ForegroundColor Red }
function Print-Info { Write-Host "[INFO] $args" -ForegroundColor Cyan }
function Print-Title { 
    Write-Host ""
    Write-Host "=" * 50 -ForegroundColor Cyan
    Write-Host "  $args" -ForegroundColor Cyan
    Write-Host "=" * 50 -ForegroundColor Cyan
    Write-Host ""
}

# Check compiler exists
function Get-Compiler {
    $paths = @(
        (Join-Path $ProjectRoot "target\release\xy.exe"),
        (Join-Path $ProjectRoot "target\debug\xy.exe")
    )
    foreach ($p in $paths) {
        if (Test-Path $p) { return $p }
    }
    return $null
}

# Run Rust unit tests
function Run-UnitTests {
    Print-Title "Rust Unit Tests"
    
    Push-Location $ProjectRoot
    try {
        Write-Host "Running: cargo test --lib" -ForegroundColor Gray
        cargo test --lib 2>&1 | ForEach-Object {
            if ($_ -match "test result: ok") {
                Print-Success "Unit tests passed"
                $script:Passed++
            } elseif ($_ -match "FAILED") {
                Print-Error "Unit tests failed"
                $script:Failed++
            }
            if ($Verbose) { Write-Host $_ -ForegroundColor Gray }
        }
        $script:Total++
    } finally {
        Pop-Location
    }
}

# Run integration tests
function Run-IntegrationTests {
    Print-Title "Integration Tests"
    
    $compiler = Get-Compiler
    if (-not $compiler) {
        Print-Info "Compiler not found. Run: cargo build"
        $script:Skipped++
        return
    }
    
    $testFiles = @(
        @{ Path = "tests\integration\operator_test.xy"; Name = "operator_test" },
        @{ Path = "tests\integration\control_flow_test.xy"; Name = "control_flow_test" },
        @{ Path = "tests\integration\list_test.xy"; Name = "list_test" },
        @{ Path = "tests\integration\match_test.xy"; Name = "match_test" },
        @{ Path = "tests\integration\async_test.xy"; Name = "async_test" },
        @{ Path = "examples\hello.xy"; Name = "hello" }
    )
    foreach ($test in $testFiles) {
        $script:Total++
        $fullPath = Join-Path $ProjectRoot $test.Path
        if (Test-Path $fullPath) {
            $outputFile = Join-Path $OutputDir "$($test.Name).ll"
            Write-Host "Testing: $($test.Name)" -ForegroundColor Gray
            
            & $compiler $fullPath --ir-pure 2>&1 | Out-File $outputFile -Encoding UTF8
            if ($LASTEXITCODE -eq 0) {
                Print-Success $test.Name
                $script:Passed++
            } else {
                Print-Error $test.Name
                $script:Failed++
            }
        } else {
            Print-Info "File not found: $($test.Path)"
            $script:Skipped++
        }
    }

}

# Run bootstrap tests
function Run-BootstrapTests {
    Print-Title "Bootstrap Tests"
    
    $compiler = Get-Compiler
    if (-not $compiler) {
        Print-Info "Compiler not found. Run: cargo build"
        $script:Skipped++
        return
    }
    
    $bootstrapFiles = @(
        @{ Path = "src\compiler_v2\xyc.xy"; Name = "xyc" },
        @{ Path = "src\compiler_v2\lexer_s.xy"; Name = "lexer" },
        @{ Path = "src\compiler_v2\parser_s.xy"; Name = "parser" },
        @{ Path = "src\compiler_v2\sema_s.xy"; Name = "sema" },
        @{ Path = "src\compiler_v2\codegen_s.xy"; Name = "codegen" },
        @{ Path = "tests\bootstrap\self_compile_test.xy"; Name = "self_compile" }
    )
    
    foreach ($test in $bootstrapFiles) {
        $script:Total++
        $fullPath = Join-Path $ProjectRoot $test.Path
        if (Test-Path $fullPath) {
            $outputFile = Join-Path $OutputDir "$($test.Name).ll"
            Write-Host "Testing: $($test.Name)" -ForegroundColor Gray
            
            & $compiler $fullPath --ir-pure 2>&1 | Out-File $outputFile -Encoding UTF8
            if ($LASTEXITCODE -eq 0) {
                Print-Success $test.Name
                $script:Passed++
            } else {
                Print-Error $test.Name
                $script:Failed++
            }
        } else {
            Print-Info "File not found: $($test.Path)"
            $script:Skipped++
        }
    }
}

# Generate report
function Generate-Report {
    Print-Title "Test Report"
    
    Write-Host "Total:  $Total" -ForegroundColor White
    Write-Host "Passed: $Passed" -ForegroundColor Green
    Write-Host "Failed: $Failed" -ForegroundColor Red
    Write-Host "Skipped: $Skipped" -ForegroundColor Yellow
    Write-Host ""
    
    if ($Failed -eq 0) {
        Print-Success "All tests passed!"
        return 0
    } else {
        Print-Error "$Failed tests failed"
        return 1
    }
}

# Main
Print-Title "Xuanyu Compiler Test Suite"
Print-Info "Project: $ProjectRoot"
Print-Info "Output: $OutputDir"

$runAll = $All -or (-not $Unit -and -not $Integration -and -not $Bootstrap)

if ($Unit -or $runAll) { Run-UnitTests }
if ($Integration -or $runAll) { Run-IntegrationTests }
if ($Bootstrap -or $runAll) { Run-BootstrapTests }

$exitCode = Generate-Report
exit $exitCode