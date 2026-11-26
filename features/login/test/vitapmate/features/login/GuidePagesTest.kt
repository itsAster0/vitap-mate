package vitapmate.features.login

import kotlin.test.Test
import androidx.compose.ui.geometry.Offset
import kotlin.test.assertEquals

class GuidePagesTest {
    private fun page(step: Int?, title: String) = GuidePage(step, title, "", null, null, null)

    @Test
    fun pagesForKeepsOrderAndSkipsUntagged() {
        val all = listOf(page(1, "add"), page(null, "old"), page(3, "reload"), page(2, "verify"), page(3, "from"))
        assertEquals(listOf("reload", "from"), pagesFor(3, all).map { it.title })
        assertEquals(listOf("add"), pagesFor(1, all).map { it.title })
        assertEquals(emptyList(), pagesFor(4, all))
    }

    @Test
    fun anOldGuideWithoutStepsShowsEveryPage() {
        val old = listOf(page(null, "add"), page(null, "verify"))
        assert(pagesFor(1, old).isEmpty())
        assertEquals(listOf("add", "verify"), pagesOrAll(1, old).map { it.title })
        val tagged = listOf(page(1, "add"), page(null, "old"))
        assertEquals(listOf("add"), pagesOrAll(1, tagged).map { it.title })
    }

    @Test
    fun theShownPageStaysInARefreshedShorterGuide() {
        assertEquals(1, shownPage(4, 2))
        assertEquals(0, shownPage(-1, 3))
        assertEquals(2, shownPage(2, 3))
        assertEquals(0, shownPage(3, 0))
    }

    @Test
    fun aZoomedScreenshotPansOnlyToItsEdges() {
        assertEquals(Offset.Zero, panWithin(Offset(50f, -50f), 1f, 100f, 200f))
        assertEquals(Offset.Zero, panWithin(Offset(50f, -50f), 0.5f, 100f, 200f))
        assertEquals(Offset.Zero, panWithin(Offset(50f, -50f), 2f, 0f, 200f))
        // At 2x, 100 × 200 overflows by 50 and 100 each side.
        assertEquals(Offset(50f, -100f), panWithin(Offset(80f, -300f), 2f, 100f, 200f))
        assertEquals(Offset(10f, 20f), panWithin(Offset(10f, 20f), 2f, 100f, 200f))
    }
}
