package app.boundless.driver.i18n

import app.boundless.rider.i18n.CatalogRiderStrings
import java.io.File
import java.util.Locale

/**
 * A test/snapshot [DriverStrings] backed by the REAL `strings.xml` files — the SHARED catalog (in
 * `:rider:shared`) **merged with** the Driver's own 4-key catalog (in `:driver:app`) — so tests and
 * the shipped copy share a single source of truth with no English drift. Reuses
 * [CatalogRiderStrings.parseValues] for the parse + Android unescaping (no duplicated XML logic); only
 * the multi-file merge is Driver-specific. Pure JVM, no `Context`/Robolectric/emulator, so the
 * Paparazzi snapshots render genuine catalog copy without Paparazzi resource resolution.
 *
 * The production resolver (`AndroidDriverStrings`, over Android `Resources`) is the deferred app shell.
 */
class CatalogDriverStrings private constructor(private val values: Map<String, String>) : DriverStrings {
    override fun string(key: String, vararg args: Any): String {
        val template = values[key] ?: error("Missing string resource '$key' in the merged catalog")
        return if (args.isEmpty()) template else String.format(Locale.ROOT, template, *args)
    }

    companion object {
        /** Reads the shared + Driver catalog paths from the `boundless.strings.path` and
         *  `boundless.strings.driver.path` system properties, which `:driver:app`'s build.gradle.kts
         *  `testOptions` points at the real strings.xml files. */
        fun fromDefaultCatalog(): CatalogDriverStrings {
            val shared = System.getProperty("boundless.strings.path")
                ?: error("system property 'boundless.strings.path' not set — see :driver:app build.gradle.kts testOptions")
            val driver = System.getProperty("boundless.strings.driver.path")
                ?: error("system property 'boundless.strings.driver.path' not set — see :driver:app build.gradle.kts testOptions")
            // Shared first, Driver second (Driver keys would override on collision, though none do).
            return CatalogDriverStrings(CatalogRiderStrings.parseValues(File(shared), File(driver)))
        }
    }
}
