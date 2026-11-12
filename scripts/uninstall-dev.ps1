#Requires -RunAsAdministrator
sc.exe stop GuidanceRuntime;Start-Sleep 2;sc.exe delete GuidanceRuntime;Remove-Item -Recurse -Force 'C:\Program Files\Guidance Runtime'
