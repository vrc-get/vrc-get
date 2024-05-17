@setlocal

@echo ALCOM vcc: protocol installer
@echo this script will register the vcc: protocol with ALCOM
@echo the function of this script will be integrated into the ALCOM itself in the future
@echo but for now, you need to run this script manually 
@echo.
@echo how to use
@echo execute this script if you've installed ALCOM to %LOCALAPPDATA%\ALCOM\ALCOM.exe.
@echo if you have changed the installation path, Drag and drop the ALCOM.exe to this script.

@echo do you actually want to continue? ctrl + c to cancel

@pause

@if "%~1"=="" (
  @set ALCOM_EXE=%LOCALAPPDATA%\ALCOM\ALCOM.exe
) else (
  @set ALCOM_EXE=%~1
)

@if not exist "%ALCOM_EXE%" (
  @echo error: ALCOM.exe not found at %ALCOM_EXE%
  @exit /b 1
)

@echo registering vcc: using alcom path %ALCOM_EXE%

@reg add HKCU\Software\Classes\vcc                    /f /v "URL Protocol" /t REG_SZ /d ""                              > NUL
@reg add HKCU\Software\Classes\vcc\DefaultIcon        /f /v ""             /t REG_SZ /d "\"%ALCOM_EXE%\",0"             > NUL
@reg add HKCU\Software\Classes\vcc\shell\open\command /f /v ""             /t REG_SZ /d "\"%ALCOM_EXE%\" link \"%%1\""  > NUL

@echo registered vcc: for ALCOM with %ALCOM_EXE%

@pause
