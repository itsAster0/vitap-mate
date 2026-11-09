package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals

class DocsTest {
    private fun doc(id: String, name: String, added: Long, opened: Long? = null) =
        Doc(id, name, "$id.pdf", "content://test/$id", DocKind.Pdf, 1, "import", added, opened)

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
    fun kindFromExtension() {
        assertEquals(DocKind.Pdf, DocKind.of("a.PDF"))
        assertEquals(DocKind.Image, DocKind.of("x.jpeg"))
        assertEquals(DocKind.Spreadsheet, DocKind.of("menu.xlsx"))
        assertEquals(DocKind.Text, DocKind.of("notes.md"))
        assertEquals(DocKind.File, DocKind.of("archive.zip"))
        assertEquals(DocKind.File, DocKind.of("noext"))
    }
}
