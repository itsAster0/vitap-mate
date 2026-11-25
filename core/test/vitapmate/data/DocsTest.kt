package vitapmate.data

import java.io.File
import java.io.IOException
import java.nio.file.Files
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class DocsTest {
    private fun doc(id: String, name: String, added: Long, opened: Long? = null, kind: DocKind = DocKind.Pdf, size: Long = 1) =
        Doc(id, name, "$id.pdf", "content://test/$id", kind, size, "import", added, opened)

    private val docs = listOf(
        doc("a", "Mess menu", added = 3, opened = 10),
        doc("b", "outing pass", added = 1),
        doc("c", "Fee receipt", added = 2, opened = 20),
        doc("d", "Calendar", added = 4),
    )

    @Test
    fun recentlyOpenedPutsNeverOpenedLastByName() {
        assertEquals(listOf("c", "a", "d", "b"), queryDocs(docs, DocSort.RecentlyOpened).map { it.id })
    }

    @Test
    fun otherSorts() {
        assertEquals(listOf("d", "a", "c", "b"), queryDocs(docs, DocSort.RecentlyAdded).map { it.id })
        assertEquals(listOf("d", "c", "a", "b"), queryDocs(docs, DocSort.NameAscending).map { it.id })
        assertEquals(listOf("b", "a", "c", "d"), queryDocs(docs, DocSort.NameDescending).map { it.id })
    }

    @Test
    fun largestAndTypeSorts() {
        val mixed = listOf(
            doc("p", "b pass", 1, kind = DocKind.Pdf, size = 50),
            doc("i", "photo", 1, kind = DocKind.Image, size = 900),
            doc("s", "menu", 1, kind = DocKind.Spreadsheet, size = 50),
            doc("q", "a notes", 1, kind = DocKind.Pdf, size = 10),
        )
        assertEquals(listOf("i", "p", "s", "q"), queryDocs(mixed, DocSort.Largest).map { it.id })
        assertEquals(listOf("q", "p", "i", "s"), queryDocs(mixed, DocSort.Type).map { it.id })
        assertEquals(listOf("q", "p"), queryDocs(mixed, DocSort.NameAscending, kind = DocKind.Pdf).map { it.id })
    }

    @Test
    fun searchIgnoresCaseAndSpaces() {
        assertEquals(listOf("a"), queryDocs(docs, DocSort.NameDescending, "  MENU ").map { it.id })
        assertEquals(listOf("c", "a", "d"), queryDocs(docs, DocSort.RecentlyOpened, "e").map { it.id })
        assertEquals(listOf("b"), queryDocs(docs, DocSort.NameAscending, "PASS").map { it.id })
        assertEquals(emptyList(), queryDocs(docs, DocSort.NameAscending, "zzz"))
    }

    @Test
    fun tiesFallBackToNameThenId() {
        val same = listOf(doc("2", "x", 1), doc("1", "x", 1), doc("3", "a", 1))
        assertEquals(listOf("3", "1", "2"), queryDocs(same, DocSort.RecentlyAdded).map { it.id })
    }

    @Test
    fun theMessMenuIsTheNewestMarkedOne() {
        val menus = docs + doc("m1", "Menu Oct", added = 5).copy(asMessMenu = true) + doc("m2", "Menu Nov", added = 9).copy(asMessMenu = true)
        assertEquals("m2", menus.newestMessMenu()?.id)
        assertEquals(null, docs.newestMessMenu())
    }

    @Test
    fun kindFromExtension() {
        assertEquals(DocKind.Pdf, DocKind.of("a.PDF"))
        assertEquals(DocKind.Image, DocKind.of("x.jpeg"))
        assertEquals(DocKind.Spreadsheet, DocKind.of("menu.xlsx"))
        assertEquals(DocKind.Text, DocKind.of("notes.md"))
        assertEquals(DocKind.File, DocKind.of("archive.zip"))
        assertEquals(DocKind.File, DocKind.of("noext"))
    }

    @Test
    fun writeAtomicallyReplacesConcurrentlyAndCleansUpOnRenameFailure() {
        val dir = Files.createTempDirectory("atomic-write").toFile()
        try {
            val target = File(dir, "state.json")
            val payloads = (1..8).map { n -> "payload-$n-" + "x".repeat(64) }
            val errors = mutableListOf<Throwable>()
            val threads = payloads.map { body ->
                thread {
                    runCatching { repeat(10) { target.writeAtomically(body.toByteArray()) } }
                        .onFailure { synchronized(errors) { errors += it } }
                }
            }
            threads.forEach { it.join() }
            assertEquals(emptyList(), errors)
            assertTrue(target.readText() in payloads)
            assertEquals(listOf("state.json"), dir.list()?.toList())

            // Rename over a non-empty directory fails and removes its sibling temp file.
            val blocked = File(dir, "blocked").apply { mkdirs() }
            File(blocked, "keep").writeText("1")
            assertFailsWith<IOException> { blocked.writeAtomically("nope".toByteArray()) }
            assertTrue(dir.listFiles().orEmpty().none { it.name.endsWith(".tmp") })
        } finally {
            dir.deleteRecursively()
        }
    }
}
