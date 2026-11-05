package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class MessMenuTest {
    private val rows = listOf(
        listOf("VIT-AP MESS MENU FOR THE MONTH OF OCTOBER", "", "", "", ""),
        listOf("Day", "Breakfast", "Lunch", "Snacks", "Dinner"),
        listOf("Thu\n1, 15, 29", "Idli", "Aloo Mutter Curry (1), Chickpea Masala (15, 29)", "Samosa (2)", "Chicken Curry (Non-Veg)"),
        listOf("", "Chutney", "Rice", "", "Curd"),
        listOf("Fri 2, 16", "Dosa", "Dal", "Tea", "Paneer"),
        listOf("Instructions", "", "", "", ""),
        listOf("1. Carry your ID card\n2. No outside food", "", "", "", ""),
    )

    @Test
    fun parsesMonthBlocksAndInstructions() {
        val menu = parseMessMenu(mapOf("Veg & Non-Veg " to rows, "Notes" to listOf(listOf("nothing"))))
        assertEquals(1, menu.sheets.size)
        val sheet = menu.sheets.single()
        assertEquals("Veg & Non-Veg", sheet.name)
        assertEquals(10, menu.month)
        assertEquals(listOf(1, 15, 29), sheet.blocks[15])
        assertEquals(listOf("Idli", "Chutney"), sheet.itemsFor(29, Meal.Breakfast).map { it.name })
        assertEquals(listOf("Carry your ID card", "No outside food"), sheet.instructions)
        assertEquals(listOf("Dosa"), sheet.itemsFor(16, Meal.Breakfast).map { it.name })
    }

    @Test
    fun resolvesDateVariantsQuantitiesAndNonVeg() {
        val sheet = parseMessMenu(mapOf("M" to rows)).sheets.single()
        assertEquals(listOf(MessItem("Aloo Mutter Curry"), MessItem("Rice")), sheet.itemsFor(1, Meal.Lunch))
        assertEquals(listOf(MessItem("Chickpea Masala"), MessItem("Rice")), sheet.itemsFor(15, Meal.Lunch))
        assertEquals(listOf(MessItem("Samosa (2)")), sheet.itemsFor(1, Meal.Snacks))
        assertEquals(MessItem("Chicken Curry", nonVeg = true), sheet.itemsFor(1, Meal.Dinner).first())
        assertNull(resolveMessItem("Kheer (1)", 15, listOf(1, 15, 29)))
    }

    @Test
    fun refusesOtherSpreadsheets() {
        assertFailsWith<MessMenuFormatException> { parseMessMenu(mapOf("S" to listOf(listOf("a", "b")))) }
    }

    @Test
    fun mealWindowsRoundTripAndFallBack() {
        assertEquals("435-540,750-840,990-1095,1155-1260", encodeMealWindows(defaultMealWindows))
        val custom = defaultMealWindows + (Meal.Lunch to MealWindow(700, 800))
        assertEquals(custom, decodeMealWindows(encodeMealWindows(custom)))
        assertEquals(defaultMealWindows, decodeMealWindows("junk"))
        assertEquals(defaultMealWindows, decodeMealWindows(null))
    }

    @Test
    fun currentOrNextMealAndYear() {
        assertEquals(Meal.Breakfast, currentOrNextMeal(0))
        assertEquals(Meal.Lunch, currentOrNextMeal(9 * 60))
        assertNull(currentOrNextMeal(21 * 60))
        assertEquals(2025, messMenuYear(10, "Menu October 2025", 2026, 11))
        assertEquals(2027, messMenuYear(1, "Menu", 2026, 11))
        assertEquals(2025, messMenuYear(12, "Menu", 2026, 2))
        assertEquals(2026, messMenuYear(null, "Menu", 2026, 2))
    }

    @Test
    fun weekdayDecidesConflictingDatesAndFillsTheFortnight() {
        // November 2026 starts on a Sunday: 10 and 24 are Tuesdays, 25 a Wednesday, 26 a Thursday.
        val rows = listOf(
            listOf("MESS MENU FOR THE MONTH OF NOVEMBER", "", "", "", ""),
            listOf("Day", "Breakfast", "Lunch", "Snacks", "Dinner"),
            listOf("Tue\n10, 25", "Dosa", "", "", ""),
            listOf("Wed / 11, 26", "Vada", "", "", ""),
            listOf("Thu      12, 26", "Poori", "", "", ""),
        )
        val sheet = parseMessMenu(mapOf("M" to rows)) { 2026 }.sheets.single()
        assertEquals(listOf("Dosa"), sheet.itemsFor(24, Meal.Breakfast).map { it.name })
        assertEquals(listOf("Vada"), sheet.itemsFor(25, Meal.Breakfast).map { it.name })
        assertEquals(listOf("Poori"), sheet.itemsFor(26, Meal.Breakfast).map { it.name })
        assertEquals(listOf(10, 24), sheet.blocks[24])
        // Without the year, the dates stay as listed and the first block keeps a duplicate.
        val raw = parseMessMenu(mapOf("M" to rows)).sheets.single()
        assertEquals(listOf("Dosa"), raw.itemsFor(25, Meal.Breakfast).map { it.name })
        assertEquals(listOf("Vada"), raw.itemsFor(26, Meal.Breakfast).map { it.name })
    }

    @Test
    fun readsTheNovember2026Menu() {
        val file = listOf("core/testResources", "testResources").map { java.io.File(it, "mess-menu-2026-11.xlsx") }.firstOrNull { it.exists() }
            ?: java.io.File(javaClass.getResource("/mess-menu-2026-11.xlsx")!!.toURI())
        val menu = parseMessMenu(readSpreadsheet(file)) { 2026 }
        assertEquals(listOf("Veg & Non-Veg", "Special"), menu.sheets.map { it.name })
        assertEquals(11, menu.month)
        val sheet = menu.sheets.first()
        // Every date of the month has exactly one block.
        assertEquals((1..30).toList(), sheet.days.keys.sorted())
        assertEquals(listOf(8, 22), sheet.blocks[8])
        val breakfast8 = sheet.itemsFor(8, Meal.Breakfast)
        assertEquals("Shavige Bath", breakfast8.first().name)
        assertEquals(MessItem("Egg Bhurji", nonVeg = true), breakfast8.last())
        assertEquals("Dahi Papdi Chat", sheet.itemsFor(8, Meal.Snacks).first().name)
        assertEquals("Ghee Podi Dosa (2 Pcs Thin)", sheet.itemsFor(24, Meal.Breakfast).first().name)
        assertEquals("Methu Vada (Big size 3pcs, if not 4pcs)", sheet.itemsFor(25, Meal.Breakfast).first().name)
        assertEquals("Methi Poori (3 Pcs standard size)", sheet.itemsFor(26, Meal.Breakfast).first().name)
        assertEquals("Onion Dosa - 2 Pcs Std Size", sheet.itemsFor(3, Meal.Breakfast).first().name)
        assertEquals("Masala Dosa - 2 Pcs Std Size", sheet.itemsFor(17, Meal.Breakfast).first().name)
        assertEquals(MessItem("Paneer Butter Masala"), sheet.itemsFor(2, Meal.Dinner).first { "Paneer" in it.name })
        assert(sheet.instructions.first().startsWith("Thick curd must be served"))
    }
}
