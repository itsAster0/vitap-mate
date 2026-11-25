package vitapmate

/**
 * Where the app is between start and the shell ([App]): one phase at a time, moved only by
 * [next]. [user] is the signed-in username for the avatar, null until read.
 */
sealed interface AppSession {
    /** The core is starting. */
    data object Booting : AppSession

    /** The core didn't start; [error] shows instead of the app. */
    data class BootFailed(val error: Throwable) : AppSession

    data object SignedOut : AppSession

    /** Signed in: the account and the saved semester pick are being read. */
    data class LoadingAccount(val user: String?) : AppSession

    /** No semester picked; [firstTime]: the setup page ([Onboarding]) follows the pick. */
    data class PickingSemester(val user: String?, val firstTime: Boolean) : AppSession

    data class Onboarding(val user: String?) : AppSession

    data class Ready(val user: String?) : AppSession
}

/** What moves [AppSession] on. */
sealed interface SessionEvent {
    /** The core started; [user] null when signed out. */
    data class Booted(val user: String?) : SessionEvent

    data class BootError(val error: Throwable) : SessionEvent

    /** The login screen signed in. */
    data object SignedIn : SessionEvent

    /** The username, read again (after a sign-in, or the account changed in place). */
    data class UserRead(val user: String?) : SessionEvent

    /** The account and the saved pick were read: [picked] a semester; [onboarded] once on this install. */
    data class AccountRead(val picked: Boolean, val onboarded: Boolean) : SessionEvent

    /** The semester pick came or went (picked in the sheet, or forgotten). */
    data class PickChanged(val picked: Boolean, val onboarded: Boolean) : SessionEvent

    data object OnboardingDone : SessionEvent

    /** Signed out, or a problem's "Sign in" sent the student to the login screen. */
    data object SignedOut : SessionEvent
}

/** The signed-in username, for the avatar; null while unknown or signed out. */
val AppSession.user: String?
    get() = when (this) {
        AppSession.Booting, is AppSession.BootFailed, AppSession.SignedOut -> null
        is AppSession.LoadingAccount -> user
        is AppSession.PickingSemester -> user
        is AppSession.Onboarding -> user
        is AppSession.Ready -> user
    }

/** Whether a student is signed in (the account may still be loading). */
val AppSession.signedIn: Boolean
    get() = this is AppSession.LoadingAccount || this is AppSession.PickingSemester || this is AppSession.Onboarding || this is AppSession.Ready

/** [s] after [e]; an event that doesn't apply leaves it as it is. */
fun next(s: AppSession, e: SessionEvent): AppSession = when (e) {
    is SessionEvent.Booted -> if (s != AppSession.Booting) s else if (e.user == null) AppSession.SignedOut else AppSession.LoadingAccount(e.user)
    is SessionEvent.BootError -> if (s == AppSession.Booting) AppSession.BootFailed(e.error) else s
    SessionEvent.SignedIn -> if (s == AppSession.SignedOut) AppSession.LoadingAccount(null) else s
    is SessionEvent.UserRead -> when (s) {
        is AppSession.LoadingAccount -> s.copy(user = e.user)
        is AppSession.PickingSemester -> s.copy(user = e.user)
        is AppSession.Onboarding -> s.copy(user = e.user)
        is AppSession.Ready -> s.copy(user = e.user)
        AppSession.Booting, is AppSession.BootFailed, AppSession.SignedOut -> s
    }
    is SessionEvent.AccountRead -> when (s) {
        is AppSession.LoadingAccount -> if (e.picked) AppSession.Ready(s.user) else AppSession.PickingSemester(s.user, firstTime = !e.onboarded)
        else -> s
    }
    is SessionEvent.PickChanged -> when {
        s is AppSession.PickingSemester && e.picked -> if (s.firstTime) AppSession.Onboarding(s.user) else AppSession.Ready(s.user)
        s is AppSession.Ready && !e.picked -> AppSession.PickingSemester(s.user, firstTime = !e.onboarded)
        else -> s
    }
    SessionEvent.OnboardingDone -> if (s is AppSession.Onboarding) AppSession.Ready(s.user) else s
    SessionEvent.SignedOut -> if (s.signedIn) AppSession.SignedOut else s
}
