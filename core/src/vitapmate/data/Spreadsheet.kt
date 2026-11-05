package vitapmate.data

import java.io.File
import java.util.zip.ZipFile
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Document
import org.w3c.dom.Element

/**
 * An .xlsx/.xlsm file as sheet name → rows of cell text, in workbook order; empty cells (and the
 * covered part of a merged range) are "". The Flutter app's `decodeSpreadsheetTables`, with the
 * platform's zip and XML parser instead of a package. Legacy .xls and .ods are refused.
 */
fun readSpreadsheet(file: File): Map<String, List<List<String>>> = ZipFile(file).use { zip ->
    val builder = DocumentBuilderFactory.newInstance().newDocumentBuilder()
    fun xml(name: String): Document? = zip.getEntry(name)?.let { e -> zip.getInputStream(e).use(builder::parse) }
    fun Element.all(tag: String): List<Element> = getElementsByTagName(tag).let { l -> List(l.length) { l.item(it) as Element } }
    fun Document.all(tag: String) = documentElement.all(tag)

    // A shared string is its runs' text; reading aids (rPh, furigana) are not part of it.
    val strings = xml("xl/sharedStrings.xml")?.all("si").orEmpty().map { si ->
        si.all("t").filter { (it.parentNode as? Element)?.tagName != "rPh" }.joinToString("") { it.textContent }
    }
    val targets = xml("xl/_rels/workbook.xml.rels")?.all("Relationship").orEmpty()
        .associate { it.getAttribute("Id") to it.getAttribute("Target").removePrefix("/xl/").removePrefix("/") }
    val sheets = xml("xl/workbook.xml")?.all("sheet").orEmpty().mapNotNull { s -> targets[s.getAttribute("r:id")]?.let { s.getAttribute("name") to "xl/$it" } }
    if (sheets.isEmpty()) error("Not an .xlsx spreadsheet.")
    sheets.associate { (name, path) ->
        val cells = sortedMapOf<Int, MutableMap<Int, String>>()
        xml(path)?.all("row").orEmpty().forEachIndexed { i, row ->
            val r = row.getAttribute("r").toIntOrNull()?.minus(1) ?: i
            row.all("c").forEachIndexed { j, c ->
                val raw = c.all("v").firstOrNull()?.textContent ?: c.all("t").joinToString("") { it.textContent }
                val text = when (c.getAttribute("t")) {
                    "s" -> raw.toIntOrNull()?.let(strings::getOrNull).orEmpty()
                    "b" -> if (raw == "1") "TRUE" else "FALSE"
                    "inlineStr", "str", "e" -> raw
                    else -> raw.removeSuffix(".0")
                }
                if (text.isNotEmpty()) cells.getOrPut(r) { mutableMapOf() }[c.getAttribute("r").takeIf { it.isNotEmpty() }?.let(::columnOf) ?: j] = text
            }
        }
        val width = (cells.values.flatMap { it.keys }.maxOrNull() ?: -1) + 1
        name to if (cells.isEmpty()) emptyList() else (0..cells.lastKey()).map { r -> List(width) { c -> cells[r]?.get(c).orEmpty() } }
    }
}

/** "AB12" → 27 (zero-based column). */
private fun columnOf(ref: String): Int = ref.takeWhile(Char::isLetter).fold(0) { n, ch -> n * 26 + (ch.uppercaseChar() - 'A' + 1) } - 1
