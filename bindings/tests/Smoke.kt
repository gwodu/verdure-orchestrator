// The SDK through its Kotlin binding on a desktop JVM (JNA loads verdure.dll /
// libverdure.dylib / libverdure.so from -Djna.library.path). The same calls an
// Android app makes, run where the library was just built — here to prove the
// JNA path on Windows, which no phone exercises.
//
//   kotlinc verdure.kt Smoke.kt -cp jna.jar;coroutines.jar -include-runtime -d smoke.jar
//   java -Djna.library.path=<folder with the library> -cp smoke.jar;jna.jar;coroutines.jar SmokeKt <assets> <data dir> [<seed>]
//
// <seed> is a folder of bundle folders (model files a vendor would ship in
// their installer). With it, the bundle is loaded and asked a question.
import java.io.File
import uniffi.verdure.*

fun main(args: Array<String>) {
    val assets = File(args[0])
    val dataDir = File(args[1])
    val seed = args.getOrNull(2)?.let { File(it) }
    fun text(name: String) = File(assets, name).readText(Charsets.UTF_8)

    val policy = PolicyConfig(networkDownloads = false, allowedHosts = emptyList(),
                              localCounters = false, encryptedStorage = true, retentionDays = null)
    val sdk = Verdure.start(text("sandbox.token").trim(), policy, dataDir.absolutePath)
    val caps = sdk.capabilities()
    check(caps.totalRamMb >= 1024uL) { "total memory reads ${caps.totalRamMb} MB" }
    check(caps.availableRamMb > 0uL && caps.availableRamMb <= caps.totalRamMb) { "available ${caps.availableRamMb} of ${caps.totalRamMb} MB" }
    check(caps.freeDiskMb > 0uL) { "free disk reads 0 MB" }
    println("device: ${caps.os} ${caps.arch}, tier ${caps.tier}, ${caps.availableRamMb}/${caps.totalRamMb} MB free, engines ${caps.backends}")

    sdk.loadManifest(text("example.json"), text("example.json.sig"), text("issuer.pub").trim())
    val bundle = sdk.recommend(Task.SCRIBE)
    println("recommend: ${bundle.id}; passed over: ${bundle.skipped}")

    try {
        sdk.load("no-such-bundle")
        error("loading a bundle that does not exist succeeded")
    } catch (e: VerdureException.NoBundle) {
        println("an unknown bundle is refused: ${e.message}")
    }

    if (seed != null) {
        val from = File(seed, bundle.id)
        val to = File(dataDir, "bundles/${bundle.id}")
        to.mkdirs()
        from.listFiles()!!.forEach { it.copyTo(File(to, it.name), overwrite = true) }
        check(sdk.isDownloaded(bundle.id)) { "files copied into place are not recognised" }
        sdk.load(bundle.id)
        val reply = sdk.chat(listOf(ChatMessage(Role.USER, "What is the capital of France? Answer with one word.")), null)
        check(reply.lowercase().contains("paris")) { "expected Paris, got: $reply" }
        println("chat: ${reply.trim()}")
        sdk.delete(DeleteScope.All)
        check(!File(dataDir, "bundles").exists() && !File(dataDir, "models").exists()) { "delete left model files behind" }
    }
    println("KOTLIN-OK")
}
