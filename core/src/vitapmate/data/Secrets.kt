package vitapmate.data

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Small secret files in the app's files dir, sealed with AES-GCM under an Android Keystore key
 * that never leaves the Keystore (one key per [name]). The file is the 12-byte IV followed by the
 * ciphertext. A file that no longer opens (key gone after a restore to a new phone) reads as null.
 */
object Secrets {
    private const val STORE = "AndroidKeyStore"
    private const val CIPHER = "AES/GCM/NoPadding"
    private const val IV = 12

    fun read(context: Context, name: String): ByteArray? {
        val file = file(context, name).takeIf { it.exists() } ?: return null
        return runCatching {
            val bytes = file.readBytes()
            Cipher.getInstance(CIPHER).run {
                init(Cipher.DECRYPT_MODE, key(name), GCMParameterSpec(128, bytes, 0, IV))
                doFinal(bytes, IV, bytes.size - IV)
            }
        }.getOrNull()
    }

    fun write(context: Context, name: String, plain: ByteArray) {
        val cipher = Cipher.getInstance(CIPHER).apply { init(Cipher.ENCRYPT_MODE, key(name)) }
        val sealed = cipher.iv + cipher.doFinal(plain)
        val file = file(context, name)
        val tmp = File(file.path + ".tmp")
        tmp.writeBytes(sealed)
        tmp.renameTo(file)
    }

    fun delete(context: Context, name: String) {
        file(context, name).delete()
    }

    private fun file(context: Context, name: String) = File(context.filesDir, "$name.sealed")

    private fun key(name: String): SecretKey {
        val alias = "vitapmate.$name"
        val store = KeyStore.getInstance(STORE).apply { load(null) }
        (store.getKey(alias, null) as? SecretKey)?.let { return it }
        val spec = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .build()
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, STORE).apply { init(spec) }.generateKey()
    }
}
