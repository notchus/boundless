package app.boundless.rider.i18n

import java.io.File
import java.util.Locale
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Element

/**
 * A test/snapshot [RiderStrings] backed by the REAL `res/values/strings.xml` — parsed directly (the
 * same file `R` is generated from), so the tests and the shipped catalog share a single source of
 * truth with no possibility of English drift. This mirrors how the iOS `RiderShared` tests resolve
 * copy via the real String Catalog (`Bundle.module`). It uses no Android `Context`, no Robolectric
 * and no emulator (pure JVM, zero new deps), which also means the Paparazzi snapshots render real
 * catalog copy WITHOUT relying on Paparazzi's own resource resolution.
 *
 * Lives in `:rider:shared`'s `main` source set so both `:rider:app` and `:driver:app` test source
 * sets can consume it single-sourced (AGP `testFixtures` would be the natural home, but KGP 2.0.21 —
 * pinned with AGP 8.4.2 / Paparazzi 1.3.5 — doesn't compile Kotlin in `testFixtures`; that landed in
 * Kotlin 2.1.0). It is **inert in production**: the shipping resolver is the `Resources`-backed
 * `AndroidRiderStrings` (the deferred app shell); `fromDefaultCatalog` reads a test-only system
 * property and is never called at runtime. The Driver's `CatalogDriverStrings` reuses [parseValues] to
 * merge this shared catalog with the Driver's own 4-key `strings.xml`. Both implement only
 * [RiderStrings.string]; the generic-fallback selection and positional substitution live once in the
 * [RiderStrings] interface.
 */
class CatalogRiderStrings private constructor(private val values: Map<String, String>) : RiderStrings {
    override fun string(key: String, vararg args: Any): String {
        val template = values[key] ?: error("Missing string resource '$key' in strings.xml")
        return if (args.isEmpty()) template else String.format(Locale.ROOT, template, *args)
    }

    companion object {
        /** Reads the catalog path from the `boundless.strings.path` system property, which an app
         *  module's build.gradle.kts `testOptions` points at the shared `:rider:shared` strings.xml. */
        fun fromDefaultCatalog(): CatalogRiderStrings {
            val path = System.getProperty("boundless.strings.path")
                ?: error(
                    "system property 'boundless.strings.path' not set — see the app build.gradle.kts testOptions",
                )
            return fromFile(File(path))
        }

        fun fromFile(file: File): CatalogRiderStrings = CatalogRiderStrings(parseValues(file))

        /**
         * Parse one or more Android `strings.xml` files into a `name → unescaped value` map, merging
         * in order (a later file overrides an earlier key, though Boundless's catalogs don't collide).
         * Public so the Driver's `CatalogDriverStrings` can merge the shared catalog + the Driver keys
         * without re-implementing the parse (the shared catalog is the single source of truth).
         */
        fun parseValues(vararg files: File): Map<String, String> {
            val map = LinkedHashMap<String, String>()
            val factory = DocumentBuilderFactory.newInstance().apply {
                isNamespaceAware = false
                // Trusted local files, but disallow DTDs anyway (no XXE surface).
                runCatching { setFeature("http://apache.org/xml/features/disallow-doctype-decl", true) }
            }
            for (file in files) {
                require(file.isFile) { "strings.xml not found at ${file.absolutePath}" }
                val doc = factory.newDocumentBuilder().parse(file)
                val nodes = doc.getElementsByTagName("string")
                for (i in 0 until nodes.length) {
                    val el = nodes.item(i) as Element
                    map[el.getAttribute("name")] = unescape(el.textContent)
                }
            }
            return map
        }

        /**
         * Apply the Android string-resource unescaping conventions (AAPT) that a JVM test must mimic
         * because it reads the raw XML, not the compiled resource: a whole-value `"…"` wrapper
         * preserves whitespace, and `\'`, `\"`, `\n`, `\t`, `\@`, `\?`, `\\` are escapes. Public so the
         * resolver test (in the app module, a separate compilation) can assert it directly.
         */
        fun unescape(raw: String): String {
            var s = raw
            if (s.length >= 2 && s.startsWith("\"") && s.endsWith("\"")) s = s.substring(1, s.length - 1)
            val out = StringBuilder(s.length)
            var i = 0
            while (i < s.length) {
                val c = s[i]
                if (c == '\\' && i + 1 < s.length) {
                    when (val n = s[i + 1]) {
                        'n' -> out.append('\n')
                        't' -> out.append('\t')
                        else -> out.append(n) // \' \" \@ \? \\ → the literal next char
                    }
                    i += 2
                } else {
                    out.append(c)
                    i++
                }
            }
            return out.toString()
        }
    }
}
