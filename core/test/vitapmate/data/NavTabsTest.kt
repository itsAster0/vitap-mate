package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals

class NavTabsTest {
    @Test
    fun defaultWhenNothingSaved() {
        assertEquals(listOf("timetable", "attendance", "more"), NavTabs.decode(null))
    }

    @Test
    fun normalizeDedupesTrimsAndCapsAtFourKeepingMore() {
        assertEquals(listOf("a", "b", "more"), NavTabs.normalize(listOf(" a", "b", "a", "", "more")))
        assertEquals(listOf("a", "b", "c", "d", "more"), NavTabs.normalize(listOf("a", "b", "c", "d", "e", "f")))
        assertEquals(listOf("a", "b", "c", "d", "more"), NavTabs.normalize(listOf("a", "b", "c", "d", "e", "more")))
        assertEquals(listOf("more", "a", "b", "c", "d"), NavTabs.normalize(listOf("more", "a", "b", "c", "d", "e")))
        assertEquals(listOf("more"), NavTabs.normalize(emptyList()))
    }

    @Test
    fun addGoesBeforeMoreAndStopsAtFive() {
        val tabs = NavTabs.DEFAULT
        val five = NavTabs.add(NavTabs.add(tabs, "marks"), "exams")
        assertEquals(listOf("timetable", "attendance", "marks", "exams", "more"), five)
        assertEquals(five, NavTabs.add(five, "grades"))
        assertEquals(tabs, NavTabs.add(tabs, "attendance"))
        assertEquals(listOf("more", "a", "b"), NavTabs.add(listOf("more", "a"), "b"))
    }

    @Test
    fun removeKeepsMore() {
        assertEquals(listOf("attendance", "more"), NavTabs.remove(NavTabs.DEFAULT, "timetable"))
        assertEquals(NavTabs.DEFAULT, NavTabs.remove(NavTabs.DEFAULT, "more"))
        assertEquals(NavTabs.DEFAULT, NavTabs.remove(NavTabs.DEFAULT, "unknown"))
    }

    @Test
    fun moveClampsAndIgnoresUnknown() {
        assertEquals(listOf("attendance", "timetable", "more"), NavTabs.move(NavTabs.DEFAULT, "timetable", 1))
        assertEquals(listOf("attendance", "more", "timetable"), NavTabs.move(NavTabs.DEFAULT, "timetable", 99))
        assertEquals(listOf("more", "timetable", "attendance"), NavTabs.move(NavTabs.DEFAULT, "more", -3))
        assertEquals(NavTabs.DEFAULT, NavTabs.move(NavTabs.DEFAULT, "nope", 0))
    }

    @Test
    fun encodeDecodeRoundTrip() {
        val tabs = listOf("attendance", "timetable", "marks", "more")
        assertEquals(tabs, NavTabs.decode(NavTabs.encode(tabs)))
        assertEquals(listOf("a", "more"), NavTabs.decode("a,,a"))
    }
}
