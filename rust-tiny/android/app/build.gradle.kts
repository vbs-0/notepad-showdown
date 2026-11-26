plugins {
    id("com.android.application")
}

android {
    namespace = "com.vbs.tinynotepad"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.vbs.tinynotepad"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "x86_64")
            isUniversalApk = false
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}
