package pe.nikescar.uad_shizuku;

interface IShellService {
    String execCommand(String command) = 1;
    void execCommandToFile(String command, String outputPath) = 2;
    void destroy() = 16777114;
}
