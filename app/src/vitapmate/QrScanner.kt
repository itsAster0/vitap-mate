package vitapmate

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.os.Parcel
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.platform.LocalContext

private const val GMS_PACKAGE = "com.google.android.gms"
private const val ACTION_SCAN_BARCODE = "com.google.android.gms.mlkit.ACTION_SCAN_BARCODE"
private const val EXTRA_CALLING_APP_NAME = "extra_calling_app_name"
private const val EXTRA_SUPPORTED_FORMATS = "extra_supported_formats"
private const val EXTRA_ALLOW_MANUAL_INPUT = "extra_allow_manual_input"
private const val EXTRA_ENABLE_AUTO_ZOOM = "extra_enable_auto_zoom"
private const val EXTRA_BARCODE_RESULT = "extra_barcode_result"
private const val EXTRA_ERROR_CODE = "extra_error_code"
private const val FORMAT_QR_CODE = 256
private const val ERROR_CANCELLED = 201
private const val SAFE_PARCEL_OBJECT_HEADER = 20293

/**
 * Launches Play Services' built-in QR code scanner directly (same Activity and SafeParcelable
 * result wire contract as `play-services-code-scanner`, without bundling ML Kit's client SDK).
 * Needs no CAMERA permission. Cancelling does nothing; unavailable phones call [onUnavailable].
 */
@Composable
internal fun rememberQrScanner(
    onResult: (String) -> Unit,
    onUnavailable: (String) -> Unit,
): () -> Unit {
    val context = LocalContext.current
    val currentOnResult by rememberUpdatedState(onResult)
    val currentOnUnavailable by rememberUpdatedState(onUnavailable)
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { res ->
        val data = res.data
        if (res.resultCode == Activity.RESULT_OK && data != null) {
            val raw = data.getByteArrayExtra(EXTRA_BARCODE_RESULT)?.let(::readBarcodeRawValue)
            if (raw != null) currentOnResult(raw) else currentOnUnavailable("empty_result")
            return@rememberLauncherForActivityResult
        }
        val code = data?.getIntExtra(EXTRA_ERROR_CODE, ERROR_CANCELLED)
        if (res.resultCode == Activity.RESULT_CANCELED && (code == null || code == ERROR_CANCELLED)) return@rememberLauncherForActivityResult
        currentOnUnavailable("error_${code ?: res.resultCode}")
    }
    return remember(context, launcher) {
        {
            val intent = Intent(ACTION_SCAN_BARCODE)
                .setPackage(GMS_PACKAGE)
                .putExtra(EXTRA_CALLING_APP_NAME, appLabel(context))
                .putExtra(EXTRA_SUPPORTED_FORMATS, FORMAT_QR_CODE)
                .putExtra(EXTRA_ALLOW_MANUAL_INPUT, false)
                .putExtra(EXTRA_ENABLE_AUTO_ZOOM, false)
            if (intent.resolveActivity(context.packageManager) == null) {
                requestBarcodeModule(context)
                currentOnUnavailable("no_scanner")
            } else {
                runCatching { launcher.launch(intent) }
                    .onFailure { currentOnUnavailable(it::class.simpleName ?: "error") }
            }
        }
    }
}

private fun appLabel(context: Context): String {
    val info = context.applicationInfo
    return if (info.labelRes != 0) context.getString(info.labelRes) else info.nonLocalizedLabel?.toString().orEmpty()
}

/** Asks Play Services to fetch the optional `barcode_ui` module in the background (best effort). */
private fun requestBarcodeModule(context: Context) {
    runCatching {
        context.sendBroadcast(
            Intent("com.google.android.gms.vision.DEPENDENCY")
                .setClassName(GMS_PACKAGE, "com.google.android.gms.vision.DependencyBroadcastReceiverProxy")
                .putExtra("com.google.android.gms.vision.DEPENDENCIES", "barcode_ui")
                .putExtra("requester_app_package", context.packageName),
        )
    }
}

/**
 * Extracts `rawValue` (SafeParcelable field #3, falling back to `displayValue` field #2) from the
 * serialized `Barcode` parcel returned by Play Services' scanner (`extra_barcode_result`).
 */
internal fun readBarcodeRawValue(bytes: ByteArray): String? {
    val parcel = Parcel.obtain()
    return try {
        parcel.unmarshall(bytes, 0, bytes.size)
        parcel.setDataPosition(0)
        val objHeader = parcel.readInt()
        if ((objHeader and 0xFFFF) != SAFE_PARCEL_OBJECT_HEADER) return null
        val objSize = readSafeParcelSize(parcel, objHeader)
        val end = parcel.dataPosition() + objSize
        if (end < parcel.dataPosition() || end > parcel.dataSize()) return null
        var rawValue: String? = null
        var displayValue: String? = null
        while (parcel.dataPosition() < end) {
            val header = parcel.readInt()
            val fieldId = header and 0xFFFF
            val size = readSafeParcelSize(parcel, header)
            val start = parcel.dataPosition()
            val next = start + size
            if (size < 0 || next > end) return null
            if (size > 0) {
                when (fieldId) {
                    2 -> displayValue = parcel.readString()
                    3 -> rawValue = parcel.readString()
                }
            }
            parcel.setDataPosition(next)
        }
        rawValue ?: displayValue
    } catch (_: Throwable) {
        null
    } finally {
        parcel.recycle()
    }
}

private fun readSafeParcelSize(parcel: Parcel, header: Int): Int =
    if ((header and -65536) != -65536) (header ushr 16) and 0xFFFF else parcel.readInt()
