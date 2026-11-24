package vitapmate.data

import android.annotation.SuppressLint
import android.content.Context
import android.provider.Settings as AndroidSettings
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
 * reinstall finds them before the core opens, so it is still the same phone to the server. The
 * account's password is sealed under a Keystore key ([AccountPassword]) that an uninstall drops
 * and a new phone never has, so after either the student types the password again. The server identity comes back only on the phone that stored it
 * (its `ANDROID_ID`, kept as [OWNER]): Block Store also moves blocks to a new phone, and two phones
 * with one device key are one device to the server, each sign-in revoking the other's. Signing out
 * drops the account. Blocking calls with short timeouts, off the main thread. Log src `backup`:
 * names and sizes only, never the contents.
 */
object AccountBackup {
    private const val DEVICE = "device.json"
    private val FILES = listOf("account.json", DEVICE)
    /** The `ANDROID_ID` of the phone [DEVICE] was stored on. */
    private const val OWNER = "device.owner"
    private const val TIMEOUT_S = 3L

    /** Per phone, app signing key and user; kept across a reinstall, new on another phone. */
    @SuppressLint("HardwareIds")
    private fun phoneId(context: Context): String? =
        AndroidSettings.Secure.getString(context.contentResolver, AndroidSettings.Secure.ANDROID_ID)

    /** Writes the blocks back when this install has no account yet (a reinstall); before the core opens. */
    fun restore(context: Context) {
        val dir = context.filesDir
        if (File(dir, "account.json").exists()) return
        runCatching {
            val request = RetrieveBytesRequest.Builder().setKeys(FILES + OWNER).build()
            val blocks = Tasks.await(Blockstore.getClient(context).retrieveBytes(request), TIMEOUT_S, TimeUnit.SECONDS).blockstoreDataMap
            val owner = blocks[OWNER]?.bytes?.decodeToString()
            val samePhone = owner != null && owner == phoneId(context)
            val restored = blocks.keys.filter { it in FILES && (it != DEVICE || samePhone) }.sorted()
            for (name in restored) File(dir, name).writeBytes(blocks.getValue(name).bytes)
            Log.info("backup", "restored", fields = mapOf("files" to restored.joinToString(","), "samePhone" to samePhone.toString()))
        }.onFailure { Log.info("backup", "nothing restored: ${it::class.simpleName}") }
    }

    /** Stores the current files (after a sign-in, and at start so a changed password or a new device key is kept). */
    fun save(context: Context) {
        val client = Blockstore.getClient(context)
        val blocks = FILES.mapNotNull { name -> File(context.filesDir, name).takeIf { it.exists() }?.let { name to it.readBytes() } }.toMutableList()
        val phone = phoneId(context)
        // The owner goes first: a device block stored without it is never restored.
        if (blocks.any { it.first == DEVICE } && phone != null) blocks.add(0, OWNER to phone.encodeToByteArray())
        val stored = blocks.filter { (name, bytes) ->
            runCatching {
                val data = StoreBytesData.Builder().setKey(name).setBytes(bytes).setShouldBackupToCloud(true).build()
                Tasks.await(client.storeBytes(data), TIMEOUT_S, TimeUnit.SECONDS)
            }.onFailure { Log.info("backup", "not stored: ${it::class.simpleName}", fields = mapOf("file" to name)) }.isSuccess
        }.map { it.first }
        Log.info("backup", "stored", fields = mapOf("files" to stored.joinToString(",")))
    }

    /** Signed out: the account goes; the server identity stays, as on the phone. */
    fun forgetAccount(context: Context) {
        runCatching {
            Tasks.await(Blockstore.getClient(context).deleteBytes(DeleteBytesRequest.Builder().setKeys(listOf("account.json")).build()), TIMEOUT_S, TimeUnit.SECONDS)
        }.onFailure { Log.info("backup", "not forgotten: ${it::class.simpleName}") }
    }
}
