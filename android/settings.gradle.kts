// Boundless Android — Gradle settings (spec 001 Android bring-up).
//
// The Android side of the monorepo. The native domain/auth logic is NOT reimplemented here:
// it is the Rust core, surfaced across UniFFI by `core/ffi-kotlin` and packaged into the
// `:core-bridge` module's AAR (the P4 "BoundlessCore"). The Compose UIs (`:rider:app` T13,
// `:driver:app` T14) render the core state machine — they hold no hand-rolled auth logic.

pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}

@Suppress("UnstableApiUsage")
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "boundless-android"

include(":core-bridge")
include(":rider:app")
include(":driver:app")
