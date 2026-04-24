import java.util.Properties
import java.io.FileInputStream
import java.util.Base64
import org.gradle.api.GradleException

fun dartDefine(name: String): String? {
    val dartDefines =
        (project.findProperty("dart-defines") as String?) ?: System.getenv("DART_DEFINES")
    if (dartDefines.isNullOrBlank()) return null

    return dartDefines
        .split(",")
        .asSequence()
        .mapNotNull { encoded ->
            runCatching {
                String(Base64.getDecoder().decode(encoded), Charsets.UTF_8)
            }.getOrNull()
        }
        .mapNotNull { entry ->
            val separatorIndex = entry.indexOf('=')
            if (separatorIndex <= 0) {
                null
            } else {
                entry.substring(0, separatorIndex) to entry.substring(separatorIndex + 1)
            }
        }
        .firstOrNull { (key, _) -> key == name }
        ?.second
}

val googleOauthClientId =
    dartDefine("GOOGLE_OAUTH_CLIENT_ID")
        ?: throw GradleException(
            "Missing GOOGLE_OAUTH_CLIENT_ID. Pass it with --dart-define or --dart-define-from-file.",
        )

if (!googleOauthClientId.endsWith(".apps.googleusercontent.com")) {
    throw GradleException(
        "GOOGLE_OAUTH_CLIENT_ID must end with .apps.googleusercontent.com.",
    )
}

val googleOauthRedirectScheme =
    "com.googleusercontent.apps.${googleOauthClientId.removeSuffix(".apps.googleusercontent.com")}"

plugins {
    id("com.android.application")
    id("kotlin-android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "com.vitap_pal.app"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = "28.2.13676358"

   compileOptions {
        // Flag to enable support for the new language APIs
        isCoreLibraryDesugaringEnabled = true
        // Sets Java compatibility to Java 11
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_11.toString()
    }
dependencies {
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.1.4")
}
    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "com.vitap_pal.app"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
        manifestPlaceholders["appAuthRedirectScheme"] = googleOauthRedirectScheme
    }
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

    buildTypes {
         debug {
            signingConfig = signingConfigs.getByName("release")
        }
        release {
            // TODO: Add your own signing config for the release build.
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("release")
        }
    }
}

flutter {
    source = "../.."
}
