package vitapmate.data

import android.content.Context
import androidx.credentials.CreatePasswordRequest
import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetPasswordOption
import androidx.credentials.PasswordCredential
import androidx.credentials.exceptions.CreateCredentialException
import androidx.credentials.exceptions.GetCredentialException
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * The VTOP username and password in the phone's password manager (Credential Manager): the
 * system draws its own sheets; cancelled or no password manager, nothing happens. [context] must
 * be the activity. Flows `password.save`, `password.pick`; log src `password`, never the password.
 */
object SavedPasswords {
    /** Offers to save (or update) [username] and [password]; outlives the screen that asks. */
    fun offerSave(context: Context, username: String, password: String) {
        MainScope().launch {
            flow("password.save") { f ->
                val error = try {
                    CredentialManager.create(context).createCredential(context, CreatePasswordRequest(username, password))
                    null
                } catch (e: CreateCredentialException) {
                    e::class.simpleName
                }
                Log.info("password", "save offered", f, mapOf("saved" to "${error == null}", "error" to (error ?: "-")))
            }
        }
    }

    /** A saved (username, password) the student picks, or null (none saved, dismissed). */
    suspend fun pick(context: Context): Pair<String, String>? = flow("password.pick") { f ->
        try {
            val request = GetCredentialRequest(listOf(GetPasswordOption()), preferImmediatelyAvailableCredentials = true)
            (CredentialManager.create(context).getCredential(context, request).credential as? PasswordCredential)
                .also { Log.info("password", "picked", f, mapOf("found" to "${it != null}")) }
                ?.let { it.id to it.password }
        } catch (e: GetCredentialException) {
            Log.info("password", "none picked", f, mapOf("error" to "${e::class.simpleName}"))
            null
        }
    }
}
