powershell -c "cp ../target/x86_64-pc-windows-msvc/release/driver.dll hv.sys"

REM Load the Visual Studio Developer stuff
call "%ProgramFiles(x86)%\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat"

REM makecert -r -pe -ss PrivateCertStore -n CN=TestDriver TestDriver.cer

REM Sign the driver
signtool sign /fd certHash /td certHash /tr http://timestamp.digicert.com /v /s PrivateCertStore /n TestDriver hv.sys
