package vitapmate.data

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.security.KeyStore
import java.util.concurrent.atomic.AtomicLong
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import vitapmate.core.PasswordCipher
import vitapmate.observability.Log

private val atomicWriteSeq = AtomicLong()

/**
 * Replaces [this] atomically via a unique sibling temp file, fsyncing before rename and removing
 * the temp file if writing or renaming fails.
 */
internal fun File.writeAtomically(bytes: ByteArray) {
    val parent = parentFile?.apply { mkdirs() }
    val tmp = File(parent, "$name.${System.nanoTime()}.${atomicWriteSeq.incrementAndGet()}.tmp")
    var moved = false
    try {
        FileOutputStream(tmp).use { out ->
            out.write(bytes)
            out.fd.sync()
        }
        if (!tmp.renameTo(this)) throw IOException("failed to replace $name")
        moved = true
    } finally {
        if (!moved) tmp.delete()
    }
}

/**
 * Small secret files in the app's files dir, sealed with AES-GCM under an Android Keystore key
 * that never leaves the Keystore (one key per [name]). The file is the 12-byte IV followed by the
 * ciphertext. A file that no longer opens (key gone after a restore to a new phone) reads as null.
 */
object Secrets {
    private const val STORE = "AndroidKeyStore"
    private const val CIPHER = "AES/GCM/NoPadding"
    private const val IV = 12
    private val keyLock = Any()

    fun read(context: Context, name: String): ByteArray? {
        val file = file(context, name).takeIf { it.exists() } ?: return null
        return runCatching { file.readBytes() }.getOrNull()?.let { open(name, it) }
    }

    fun write(context: Context, name: String, plain: ByteArray) {
        val sealed = seal(name, plain)
        file(context, name).writeAtomically(sealed)
    }

    fun delete(context: Context, name: String) {
        file(context, name).delete()
    }

    /** [plain] sealed under [name]'s key: the IV, then the ciphertext. */
    fun seal(name: String, plain: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(CIPHER).apply { init(Cipher.ENCRYPT_MODE, key(name)) }
        return cipher.iv + cipher.doFinal(plain)
    }

    /** What [seal] gave back opened; null when [name]'s key can't open it (another phone's). */
    fun open(name: String, sealed: ByteArray): ByteArray? = runCatching {
        Cipher.getInstance(CIPHER).run {
            init(Cipher.DECRYPT_MODE, key(name), GCMParameterSpec(128, sealed, 0, IV))
            doFinal(sealed, IV, sealed.size - IV)
        }
    }.getOrNull()

    private fun file(context: Context, name: String) = File(context.filesDir, "$name.sealed")

    private fun key(name: String): SecretKey = synchronized(keyLock) {
        val alias = "vitapmate.$name"
        val store = KeyStore.getInstance(STORE).apply { load(null) }
        (store.getKey(alias, null) as? SecretKey)?.let { return it }
        val spec = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .build()
        KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, STORE).apply { init(spec) }.generateKey()
    }
}

/**
 * Rust's saved VTOP password (`account.json`), sealed under its own Keystore key and Base64'd
 * for the JSON. Restored onto another phone (or after a reinstall, which drops the
 * key) it doesn't open, and Rust asks for the password again. A failure must not reach Rust (it
 * would abort), so it is logged by class name only, never with the password or the sealed text.
 */
object AccountPassword : PasswordCipher {
    private const val KEY = "account.password"

    override fun seal(plain: String): String? = runCatching { Base64.encodeToString(Secrets.seal(KEY, plain.encodeToByteArray()), Base64.NO_WRAP) }
        .onFailure { Log.warn("secrets", "password not sealed: ${it::class.simpleName}") }
        .getOrNull()

    override fun open(sealed: String): String? = runCatching { Base64.decode(sealed, Base64.NO_WRAP) }.getOrNull()
        ?.let { Secrets.open(KEY, it) }
        ?.decodeToString()
}
