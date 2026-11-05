package vitapmate.features.coursepage

import androidx.compose.foundation.layout.Column
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import vitapmate.core.CoursePart

/** "CSE2001 · Theory" over the course title ([large] above the section list). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CourseTitle(course: CoursePart, modifier: Modifier = Modifier, large: Boolean = false) {
    Column(modifier) {
        Text(
            listOf(course.code, course.label).filter { it.isNotBlank() }.joinToString(" · "),
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(course.title, style = if (large) MaterialTheme.typography.headlineSmallEmphasized else MaterialTheme.typography.titleLargeEmphasized)
    }
}
