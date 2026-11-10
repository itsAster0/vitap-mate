package vitapmate.data

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.pdf.PdfDocument
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.print.PageRange
import android.print.PrintAttributes
import android.print.PrintDocumentAdapter
import android.print.PrintDocumentInfo
import android.print.PrintManager
import android.text.TextPaint
import android.text.TextUtils
import androidx.core.content.FileProvider
import java.io.File
import java.io.FileOutputStream

/** One table of a PDF: [title] (may be empty) over [header] and [rows]; [weights] share the page's width between the columns. */
class PdfTable(val title: String, val header: List<String>, val weights: List<Float>, val rows: List<List<String>>)

/** Tables as an A4 PDF in the cache's share folder (the `.logs` FileProvider), then shared or printed. */
object PdfExport {
    // A4 in points.
    private const val W = 595
    private const val H = 842
    private const val MARGIN = 40f
    private const val ROW = 20f

    /** Draws [title], the [lines] under it and [tables] into `cache/share/<name>.pdf`. Not for the main thread. */
    fun write(context: Context, name: String, title: String, lines: List<String>, tables: List<PdfTable>): File {
        val doc = PdfDocument()
        val text = TextPaint(Paint.ANTI_ALIAS_FLAG)
        val rule = Paint().apply { color = 0xFFD0D0D6.toInt(); strokeWidth = 0.5f }
        val width = W - 2 * MARGIN
        var page: PdfDocument.Page? = null
        var pages = 0
        var y = 0f
        // Room for [height] more, on a new page when this one is full.
        fun room(height: Float) {
            if (page != null && y + height <= H - MARGIN) return
            page?.let(doc::finishPage)
            page = doc.startPage(PdfDocument.PageInfo.Builder(W, H, ++pages).create())
            y = MARGIN
        }
        fun draw(s: String, size: Float, bold: Boolean, color: Int, x: Float = MARGIN, w: Float = width) {
            text.textSize = size
            text.typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
            text.color = color
            page!!.canvas.drawText(TextUtils.ellipsize(s, text, w, TextUtils.TruncateAt.END).toString(), x, y, text)
        }
        fun row(cells: List<String>, weights: List<Float>, header: Boolean) {
            room(ROW)
            y += ROW
            var x = MARGIN
            cells.forEachIndexed { i, cell ->
                val w = width * weights[i] / weights.sum()
                draw(cell, if (header) 8.5f else 9.5f, header, if (header) 0xFF6A6A75.toInt() else 0xFF1C1C1F.toInt(), x, w - 8)
                x += w
            }
            page!!.canvas.drawLine(MARGIN, y + 6, W - MARGIN, y + 6, rule)
        }
        room(0f)
        y += 22
        draw(title, 20f, true, 0xFF1C1C1F.toInt())
        lines.forEach { y += 15; draw(it, 10f, false, 0xFF6A6A75.toInt()) }
        tables.forEach { t ->
            // A table's title stays with its header and first row.
            room(26 + ROW * 2)
            if (t.title.isNotEmpty()) {
                y += 26
                draw(t.title, 12f, true, 0xFF3F3F46.toInt())
            }
            row(t.header, t.weights, true)
            t.rows.forEach { row(it, t.weights, false) }
        }
        page?.let(doc::finishPage)
        val file = File(context.cacheDir, "share").apply { mkdirs() }.resolve("$name.pdf")
        FileOutputStream(file).use(doc::writeTo)
        doc.close()
        return file
    }

    /** [file] through the share sheet. */
    fun share(context: Context, file: File) {
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.logs", file)
        val send = Intent(Intent.ACTION_SEND).setType("application/pdf").putExtra(Intent.EXTRA_STREAM, uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        send.clipData = ClipData.newRawUri(file.name, uri)
        context.startActivity(Intent.createChooser(send, file.name))
    }

    /** [file] in Android's print dialog; [context] is the activity. */
    fun print(context: Context, file: File) {
        context.getSystemService(PrintManager::class.java).print(
            file.nameWithoutExtension,
            object : PrintDocumentAdapter() {
                override fun onLayout(old: PrintAttributes?, new: PrintAttributes, signal: CancellationSignal?, callback: LayoutResultCallback, extras: Bundle?) =
                    callback.onLayoutFinished(PrintDocumentInfo.Builder(file.name).setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT).build(), old != new)

                override fun onWrite(pages: Array<PageRange>, destination: ParcelFileDescriptor, signal: CancellationSignal?, callback: WriteResultCallback) {
                    FileOutputStream(destination.fileDescriptor).use { out -> file.inputStream().use { it.copyTo(out) } }
                    callback.onWriteFinished(arrayOf(PageRange.ALL_PAGES))
                }
            },
            null,
        )
    }
}
