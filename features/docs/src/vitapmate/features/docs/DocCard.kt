package vitapmate.features.docs

import vitapmate.ui.rememberHaptics
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.border
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.toShape
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.File
import com.composables.icons.lucide.FileCode
import com.composables.icons.lucide.FileSpreadsheet
import com.composables.icons.lucide.FileText
import com.composables.icons.lucide.Image
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Utensils
import kotlinx.coroutines.delay
import vitapmate.data.Doc
import vitapmate.data.DocKind
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.ToneBadge
import vitapmate.ui.neutralTone

/** Icon and tone for [doc]'s kind (the Flutter app's `visualFor`). */
@Composable
private fun visualOf(doc: Doc): Pair<Tone, ImageVector> {
    val t = AppTheme.tones
    if (doc.asMessMenu) return t.warning to Lucide.Utensils
    return when (doc.kind) {
        DocKind.Pdf -> t.danger to Lucide.FileText
        DocKind.Image -> t.accentTone to Lucide.Image
        DocKind.Spreadsheet -> t.success to Lucide.FileSpreadsheet
        DocKind.Text -> t.lab to Lucide.FileCode
        DocKind.File -> neutralTone to Lucide.File
    }
}

private fun kindLabel(doc: Doc) = if (doc.asMessMenu) "Menu" else when (doc.kind) {
    DocKind.Pdf -> "PDF"
    DocKind.Image -> "Image"
    DocKind.Spreadsheet -> "Sheet"
    DocKind.Text -> "Text"
    DocKind.File -> "File"
}

private fun openedLabel(ms: Long?): String {
    if (ms == null || ms == 0L) return "never opened"
    val mins = (System.currentTimeMillis() - ms) / 60_000
    return when {
        mins < 1 -> "opened just now"
        mins < 60 -> "opened ${mins}m ago"
        mins < 60 * 24 -> "opened ${mins / 60}h ago"
        else -> "opened ${mins / 60 / 24}d ago"
    }
}

@OptIn(ExperimentalFoundationApi::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun DocCard(doc: Doc, index: Int, modifier: Modifier, selected: Boolean, onOpen: () -> Unit, onLongPress: () -> Unit) {
    val (tone, icon) = visualOf(doc)
    val haptics = rememberHaptics()
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val press by androidx.compose.animation.core.animateFloatAsState(if (pressed || selected) 0.94f else 1f, spring(Spring.DampingRatioMediumBouncy, Spring.StiffnessMedium))
    val ring by animateColorAsState(if (selected) MaterialTheme.colorScheme.primary else Color.Transparent)
    // Cards rise in one after another the first time the grid shows.
    val enter = remember { Animatable(0f) }
    LaunchedEffect(Unit) {
        delay((index.coerceAtMost(10) * 40).toLong())
        enter.animateTo(1f, spring(Spring.DampingRatioLowBouncy, Spring.StiffnessMediumLow))
    }
    Column(
        modifier.height(152.dp).fillMaxWidth()
            .graphicsLayer {
                alpha = enter.value.coerceIn(0f, 1f)
                translationY = (1 - enter.value) * 40f
                scaleX = press
                scaleY = press
            }
            .clip(RoundedCornerShape(26.dp))
            .background(AppTheme.colors.card)
            .border(3.dp, ring, RoundedCornerShape(26.dp))
            .combinedClickable(interaction, null, onClick = onOpen, onLongClick = {
                haptics.longPress()
                onLongPress()
            })
            .testTag("docs.card.${doc.id}"),
    ) {
        // A band in the kind's tone, so kinds tell apart across the grid; the icon in a shape
        // (its own for the mess menu and for images).
        Row(Modifier.fillMaxWidth().height(66.dp).background(tone.subtle).padding(horizontal = 14.dp), verticalAlignment = Alignment.CenterVertically) {
            val shape = when {
                doc.asMessMenu -> MaterialShapes.Cookie9Sided.toShape()
                doc.kind == DocKind.Image -> MaterialShapes.Cookie7Sided.toShape()
                else -> CircleShape
            }
            Box(Modifier.size(42.dp).background(MaterialTheme.colorScheme.surfaceContainerLowest.copy(alpha = 0.7f), shape), Alignment.Center) {
                Icon(icon, null, Modifier.size(22.dp), tint = tone.base)
            }
            Spacer(Modifier.weight(1f))
            if (selected) {
                Box(Modifier.size(28.dp).background(MaterialTheme.colorScheme.primary, CircleShape), Alignment.Center) {
                    Icon(Lucide.Check, "Selected", Modifier.size(18.dp), tint = MaterialTheme.colorScheme.onPrimary)
                }
            } else {
                ToneBadge(kindLabel(doc), tone = Tone(tone.base, MaterialTheme.colorScheme.surfaceContainerLowest.copy(alpha = 0.7f), tone.onSubtle))
            }
        }
        Column(Modifier.weight(1f).padding(start = 14.dp, top = 10.dp, end = 14.dp, bottom = 14.dp)) {
            Text(doc.name, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.height(Space.xs))
            Text(openedLabel(doc.lastOpenedAt), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1)
        }
    }
}

/**
 * "Continue viewing": the last opened document in its kind's tone, live in a short window, with
 * Full screen under it; header and window are one stack.
 */
@Composable
internal fun Recent(doc: Doc, onOpen: () -> Unit) {
    val (tone, icon) = visualOf(doc)
    val inner = RoundedCornerShape(6.dp)
    Column(
        Modifier.fillMaxWidth().padding(bottom = Space.xs).clip(MaterialTheme.shapes.extraLarge).testTag("docs.recent"),
        verticalArrangement = Arrangement.spacedBy(3.dp),
    ) {
        Row(Modifier.fillMaxWidth().background(tone.subtle, inner).padding(horizontal = Space.lg, vertical = Space.md), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(46.dp).background(MaterialTheme.colorScheme.surfaceContainerLowest.copy(alpha = 0.7f), RoundedCornerShape(15.dp)), Alignment.Center) {
                Icon(icon, null, Modifier.size(22.dp), tint = tone.base)
            }
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text("Continue viewing", style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.SemiBold, color = tone.onSubtle)
                Text(doc.name, style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        Column(Modifier.fillMaxWidth().background(AppTheme.colors.card, inner)) {
            Box(Modifier.fillMaxWidth().height(210.dp)) { DocContent(doc, embedded = true) }
            Button(
                onOpen,
                Modifier.fillMaxWidth().padding(start = Space.md, end = Space.md, bottom = Space.md).height(46.dp).testTag("docs.recent.open"),
                colors = ButtonDefaults.buttonColors(containerColor = tone.base, contentColor = AppTheme.colors.background),
            ) {
                Icon(Lucide.ExternalLink, null, Modifier.size(16.dp))
                Spacer(Modifier.width(Space.sm))
                Text("Full screen")
            }
        }
    }
}
