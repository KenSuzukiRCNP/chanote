#!/usr/bin/env python3
"""Patch src-tauri/gen/android/app/build.gradle.kts so release builds are signed with the
keystore described in src-tauri/gen/android/keystore.properties (keyAlias, password,
storeFile). This is the recipe from the Tauri docs, applied by CI; run it by hand after
`npm run tauri android init` if you sign locally. Idempotent."""
import pathlib, re, sys

root = pathlib.Path(__file__).resolve().parent.parent
gradle = root / "src-tauri/gen/android/app/build.gradle.kts"
if not gradle.exists():
    sys.exit(f"{gradle} not found — run `npm run tauri android init` first")
s = gradle.read_text()
if "keystore.properties" in s:
    print("already patched")
    sys.exit(0)

s = "import java.util.Properties\nimport java.io.FileInputStream\n" + s
signing = '''
    signingConfigs {
        create("release") {
            val keystorePropertiesFile = rootProject.file("keystore.properties")
            val keystoreProperties = Properties()
            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(FileInputStream(keystorePropertiesFile))
            }
            keyAlias = keystoreProperties["keyAlias"] as String
            keyPassword = keystoreProperties["password"] as String
            storeFile = file(keystoreProperties["storeFile"] as String)
            storePassword = keystoreProperties["password"] as String
        }
    }
'''
s, n = re.subn(r"(\nandroid\s*\{\n)", r"\1" + signing, s, count=1)
if n != 1:
    sys.exit("could not find the android { } block")
s, n = re.subn(r'(getByName\("release"\)\s*\{\n)', r'\1            signingConfig = signingConfigs.getByName("release")\n', s, count=1)
if n != 1:
    sys.exit('could not find buildTypes.getByName("release")')
gradle.write_text(s)
print("patched", gradle)
