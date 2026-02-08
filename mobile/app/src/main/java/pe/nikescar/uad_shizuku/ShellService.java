package pe.nikescar.uad_shizuku;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.FileWriter;
import java.io.InputStreamReader;

public class ShellService extends IShellService.Stub {

    private Process mProcess = null;

    @Override
    public String execCommand(String command) {
        StringBuilder output = new StringBuilder();
        try {
            mProcess = Runtime.getRuntime().exec(
                new String[]{"sh", "-c", command}, null, null
            );

            BufferedReader stdout = new BufferedReader(
                new InputStreamReader(mProcess.getInputStream())
            );
            BufferedReader stderr = new BufferedReader(
                new InputStreamReader(mProcess.getErrorStream())
            );

            String line;
            while ((line = stdout.readLine()) != null) {
                output.append(line).append("\n");
            }
            while ((line = stderr.readLine()) != null) {
                output.append(line).append("\n");
            }

            mProcess.waitFor();
        } catch (Exception e) {
            output.append("ERROR: ").append(e.getMessage()).append("\n");
        } finally {
            if (mProcess != null) {
                mProcess.destroy();
                mProcess = null;
            }
        }
        return output.toString();
    }

    @Override
    public void execCommandToFile(String command, String outputPath) {
        try {
            mProcess = Runtime.getRuntime().exec(
                new String[]{"sh", "-c", command}, null, null
            );

            BufferedReader stdout = new BufferedReader(
                new InputStreamReader(mProcess.getInputStream())
            );
            BufferedReader stderr = new BufferedReader(
                new InputStreamReader(mProcess.getErrorStream())
            );

            try (BufferedWriter writer = new BufferedWriter(new FileWriter(outputPath))) {
                String line;
                while ((line = stdout.readLine()) != null) {
                    writer.write(line);
                    writer.newLine();
                }
                while ((line = stderr.readLine()) != null) {
                    writer.write(line);
                    writer.newLine();
                }
            }

            mProcess.waitFor();
        } catch (Exception e) {
            try (BufferedWriter writer = new BufferedWriter(new FileWriter(outputPath))) {
                writer.write("ERROR: " + e.getMessage());
            } catch (Exception ignored) {}
        } finally {
            if (mProcess != null) {
                mProcess.destroy();
                mProcess = null;
            }
        }
    }

    @Override
    public void destroy() {
        if (mProcess != null) {
            mProcess.destroy();
            mProcess = null;
        }
    }
}
