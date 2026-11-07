# Windows build (untested)

The x86_64 ZIP contains `ripple.exe` and the license notices. Extract it before launching. It targets current 64-bit Windows and uses the Microsoft Visual C++ v14 runtime, including VCRUNTIME140.dll. Install the current supported x64 redistributable from Microsoft's official page if it is not installed:

https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist

Compilation and core tests ran on the Windows CI runner. No interactive Windows GUI verification has been performed. Saved sign-in persistence is currently available only on macOS; on Windows it lasts for the running process.
