@echo off
setlocal enabledelayedexpansion

REM Get script directory (equivalent to BASE_DIR)
set BASE_DIR=%~dp0

REM Remove trailing backslash
if "%BASE_DIR:~-1%"=="\" set BASE_DIR=%BASE_DIR:~0,-1%

REM Default values (arguments)
set INPUT_FILE=%~1
if "%INPUT_FILE%"=="" set INPUT_FILE=C:\Users\gusta\Documents\mei\cn\BookCloud\data\dataset\compiled_books.csv

set MAX_ROWS=%~2
if "%MAX_ROWS%"=="" set MAX_ROWS=0

set CHUNKSIZE=%~3
if "%CHUNKSIZE%"=="" set CHUNKSIZE=100000

REM Python binary (assumes python in PATH)
set PYTHON_BIN=py

set OUTPUT_DIR=C:\Users\Administrador\OneDrive\Ambiente de Trabalho\Universidade\FCUL\Mestrado\1ºAno\2ºSemestre\CN\BookCloud\data\data_clean\normalized_out

echo ========================================
echo REBUILD NORMALIZED OUTPUTS FROM COMPILED
echo ========================================
echo INPUT_FILE=%INPUT_FILE%
echo OUTPUT_DIR=%OUTPUT_DIR%
echo MAX_ROWS=%MAX_ROWS%
echo CHUNKSIZE=%CHUNKSIZE%
echo ========================================

REM Remove output directory if exists
if exist "%OUTPUT_DIR%" rmdir /s /q "%OUTPUT_DIR%"

REM Create output directory
mkdir "%OUTPUT_DIR%"

REM Run Python script
"%PYTHON_BIN%" "%BASE_DIR%\create_normalized_from_compiled.py" ^
  --input "%INPUT_FILE%" ^
  --output "%OUTPUT_DIR%" ^
  --chunksize %CHUNKSIZE% ^
  --max-rows %MAX_ROWS%

endlocal