package vitapmate.ui

import android.content.ClipData
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import kotlinx.coroutines.launch

/** Copies a value with a toast. Values are never logged. */
@Composable
fun rememberCopy(): (String) -> Unit {
    val clipboard = LocalClipboard.current
    val scope = rememberCoroutineScope()
    return { value ->
        scope.launch {
            clipboard.setClipEntry(ClipEntry(ClipData.newPlainText("vitapmate", value)))
            showToast("Copied", value, tone = ToastTone.Success)
        }
    }
}

/** Long press copies [value] (with a toast); a tap runs [onTap]. Values are never logged. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun Modifier.copyable(value: String, onTap: (() -> Unit)? = null): Modifier {
    val copy = rememberCopy()
    val haptics = rememberHaptics()
    return combinedClickable(onClick = { onTap?.invoke() }, onLongClick = {
        haptics.longPress()
        copy(value)
    })
}
