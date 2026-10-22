package cloud.khor.files

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Intent
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class OpenArgs { lateinit var path: String }

@TauriPlugin
class KhorFilesPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun open(invoke: Invoke) {
        val args = invoke.parseArgs(OpenArgs::class.java)
        activity.runOnUiThread {
            try {
                val file = File(args.path).canonicalFile
                val roots = listOf(activity.cacheDir.canonicalFile, activity.filesDir.canonicalFile)
                if (!file.isFile || roots.none { file.path.startsWith(it.path + File.separator) }) {
                    invoke.reject("File is outside the app cache")
                    return@runOnUiThread
                }
                val uri = FileProvider.getUriForFile(activity, activity.packageName + ".khor.files", file)
                val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase()) ?: "application/octet-stream"
                val intent = Intent(Intent.ACTION_VIEW).apply {
                    setDataAndType(uri, mime)
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                    clipData = ClipData.newRawUri(file.name, uri)
                }
                activity.startActivity(intent)
                invoke.resolve(JSObject())
            } catch (e: ActivityNotFoundException) {
                invoke.reject("No app can open this file")
            } catch (e: Exception) {
                invoke.reject(e.message ?: "File opening failed")
            }
        }
    }
}
