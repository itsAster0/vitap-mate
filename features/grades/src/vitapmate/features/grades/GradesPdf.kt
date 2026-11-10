package vitapmate.features.grades

import android.content.Context
import java.io.File
import vitapmate.data.AppCore
import vitapmate.data.PdfExport
import vitapmate.data.PdfTable

/** The saved grade history as a PDF: CGPA and credits, then a table for each exam session. Null without a history. */
internal suspend fun gradesPdf(context: Context, flow: String): File? {
    val h = AppCore.await().cachedGradeHistory(flow) ?: return null
    return PdfExport.write(
        context,
        "grade-history",
        "Grade history",
        listOf(
            listOf(h.name, h.regNo).filter { it.isNotEmpty() }.joinToString("  ·  "),
            "CGPA ${h.cgpa}  ·  ${h.creditsEarned} of ${h.creditsRegistered} credits earned  ·  ${h.courseCount} courses",
        ),
        h.sessions.map { s ->
            PdfTable(
                listOfNotNull(s.label, s.gpa?.let { "GPA ${it.fixed2()}" }).joinToString("  ·  "),
                listOf("CODE", "COURSE", "TYPE", "CREDITS", "GRADE"),
                listOf(1.3f, 5f, 0.8f, 1f, 0.8f),
                s.courses.map { c -> listOf(c.code, c.title, c.distribution, c.credits?.let { "%.1f".format(it) }.orEmpty(), c.grade) },
            )
        },
    )
}
