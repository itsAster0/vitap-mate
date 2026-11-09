package vitapmate.data

import android.content.Context
import com.google.android.gms.auth.blockstore.Blockstore
import com.google.android.gms.auth.blockstore.DeleteBytesRequest
import com.google.android.gms.auth.blockstore.RetrieveBytesRequest
import com.google.android.gms.auth.blockstore.StoreBytesData
import com.google.android.gms.tasks.Tasks
import java.io.File
import java.util.concurrent.TimeUnit
import vitapmate.observability.Log

/**
 * The sign-in kept across reinstalls with Play services' Block Store: the saved account
 * (`account.json`) and this phone's server identity (`device.json`), end-to-end encrypted in the
 * Google backup when the phone has a screen lock, and kept on the phone through an uninstall. A
 * reinstall finds them before the core opens, so the student is still signed in and still the
 * same phone to the server. Signing out drops the account. Blocking calls with short timeouts, off
 * the main thread. Log src `backup`: names and sizes only, never the contents.
 */
object AccountBackup {
    private val FILES = listOf("account.json", "device.json")
    private const val TIMEOUT_S = 3L

    /** Writes the blocks back when this install has no account yet (a reinstall); before the core opens. */
    fun restore(context: Context) {
        val dir = context.filesDir
        if (File(dir, "account.json").exists()) return
        runCatching {
            val request = RetrieveBytesRequest.Builder().setKeys(FILES).build()
            val blocks = Tasks.await(Blockstore.getClient(context).retrieveBytes(request), TIMEOUT_S, TimeUnit.SECONDS).blockstoreDataMap
            for ((name, block) in blocks) if (name in FILES) File(dir, name).writeBytes(block.bytes)
            Log.info("backup", "restored", fields = mapOf("files" to blocks.keys.sorted().joinToString(",")))
        }.onFailure { Log.info("backup", "nothing restored: ${it::class.simpleName}") }
    }

    /** Stores the current files (after a sign-in, and at start so a changed password or a new device key is kept). */
    fun save(context: Context) {
        val client = Blockstore.getClient(context)
        val stored = FILES.filter { name ->
            val file = File(context.filesDir, name).takeIf { it.exists() } ?: return@filter false
            runCatching {
                val data = StoreBytesData.Builder().setKey(name).setBytes(file.readBytes()).setShouldBackupToCloud(true).build()
                Tasks.await(client.storeBytes(data), TIMEOUT_S, TimeUnit.SECONDS)
            }.onFailure { Log.info("backup", "not stored: ${it::class.simpleName}", fields = mapOf("file" to name)) }.isSuccess
        }
        Log.info("backup", "stored", fields = mapOf("files" to stored.joinToString(",")))
    }

    /** Signed out: the account goes; the server identity stays, as on the phone. */
    fun forgetAccount(context: Context) {
        runCatching {
            Tasks.await(Blockstore.getClient(context).deleteBytes(DeleteBytesRequest.Builder().setKeys(listOf("account.json")).build()), TIMEOUT_S, TimeUnit.SECONDS)
        }.onFailure { Log.info("backup", "not forgotten: ${it::class.simpleName}") }
    }
}
