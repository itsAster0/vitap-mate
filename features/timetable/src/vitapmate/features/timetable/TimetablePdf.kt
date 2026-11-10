package vitapmate.features.timetable

import android.content.Context
import android.text.format.DateFormat
import java.io.File
import vitapmate.core.ClassKind
import vitapmate.data.AppCore
import vitapmate.data.PdfExport
import vitapmate.data.PdfTable
import vitapmate.data.SemesterChoice
import vitapmate.ui.clock

private val DAYS = listOf("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday")

/** The saved weekly timetable as a PDF, a table for each class day. Null without classes. */
internal suspend fun timetablePdf(context: Context, flow: String): File? {
    val tt = AppCore.await().cachedTimetable(flow)?.takeIf { it.slots.isNotEmpty() } ?: return null
    val h24 = DateFormat.is24HourFormat(context)
    return PdfExport.write(
        context,
        "timetable",
        "Timetable",
        listOfNotNull(SemesterChoice.semesters?.items?.firstOrNull { it.id == tt.semesterId }?.name, "Times are India time (IST)"),
        tt.slots.groupBy { it.day }.toSortedMap().map { (day, slots) ->
            PdfTable(
                DAYS.getOrElse(day) { "Day ${day + 1}" },
                listOf("TIME", "COURSE", "CODE", "SLOT", "ROOM", "FACULTY"),
                listOf(2.7f, 3.8f, 1.2f, 1.1f, 1.5f, 2.4f),
                slots.sortedBy { it.start }.map { s ->
                    listOf(
                        "${clock(s.start, h24)} – ${clock(s.end, h24)}",
                        if (s.kind == ClassKind.LAB) "${s.name} (Lab)" else s.name,
                        s.code,
                        s.slot,
                        listOf(s.room, s.block).filter { it.isNotEmpty() }.joinToString(" "),
                        s.faculty,
                    )
                },
            )
        },
    )
}
