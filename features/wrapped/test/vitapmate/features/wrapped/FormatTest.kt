package vitapmate.features.wrapped

import java.time.LocalDate
import kotlin.test.Test
import kotlin.test.assertEquals

class FormatTest {
    @Test
    fun counts() {
        assertEquals("1 class", count(1, "class", "classes"))
        assertEquals("3 classes", count(3, "class", "classes"))
        assertEquals("0 days", count(0, "day"))
        assertEquals("1 day", count(1, "day"))
    }

    @Test
    fun gpaAndCredits() {
        assertEquals("9.00", gpa(9.0))
        assertEquals("8.57", gpa(8.5714))
        assertEquals("24", credits(24.0))
        assertEquals("0", credits(0.0))
        assertEquals("24.5", credits(24.5))
    }

    @Test
    fun names() {
        assertEquals("Guntur", titled("GUNTUR"))
        assertEquals("New Delhi", titled("NEW DELHI"))
        assertEquals("McDonald", titled("McDonald"))
        assertEquals("", titled(""))
        assertEquals("Fall", shortName("Fall Semester 2026-27"))
        assertEquals("Winter", shortName("  Winter Semester 2026-27"))
        assertEquals("Summer", shortName("Summer"))
    }

    @Test
    fun dates() {
        assertEquals("1 Jan", day(0))
        val d = LocalDate.of(2026, 9, 15).toEpochDay().toInt()
        assertEquals("15 Sep", day(d))
        assertEquals("September", month(d))
    }
}
