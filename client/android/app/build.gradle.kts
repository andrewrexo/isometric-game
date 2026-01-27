plugins {
    id("com.android.application")
}

// Task to build Rust library with cargo-ndk
tasks.register<Exec>("buildRustLibrary") {
    workingDir = file("../..")
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-o", "android/app/src/main/jniLibs",
        "build", "--release"
    )
}

// Make sure Rust library is built before assembling
tasks.matching { it.name.startsWith("assemble") || it.name.startsWith("bundle") }.configureEach {
    dependsOn("buildRustLibrary")
}

android {
    namespace = "com.newaeven.game"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.newaeven.game"
        minSdk = 24
        targetSdk = 34
        versionCode = 4
        versionName = "0.1.3"

        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    sourceSets {
        getByName("main") {
            // Point to the jniLibs folder where cargo-ndk outputs .so files
            jniLibs.srcDirs("src/main/jniLibs")
            // Point to assets in the client folder
            assets.srcDirs("../../assets")
        }
    }

    packaging {
        jniLibs {
            // Extract native libs during install to avoid alignment issues
            useLegacyPackaging = true
        }
    }
}
