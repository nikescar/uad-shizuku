package pe.nikescar.uad_shizuku;

interface IShellCallback {
    void onOutput(String line);
    void onComplete(int exitCode);
}
