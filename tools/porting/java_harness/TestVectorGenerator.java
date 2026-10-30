package tools.porting;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;

/**
 * Generatore di vettori di test (test vectors) per il testing differenziale Java <-> Rust (Fase 3, T3.1 e T3.2).
 * Riproduce con fedeltà algoritmica le funzioni pure di Nvidium e Distant Horizons con seed fissato e casi limite.
 */
public class TestVectorGenerator {

    private static final int SEED = 42;

    // --- Algoritmi Nvidium (da NvidiumCompactChunkVertex.java) ---
    private static final int POSITION_MAX_VALUE = 65536;
    private static final int TEXTURE_MAX_VALUE = 32768;
    private static final float MODEL_ORIGIN = 8.0f;
    private static final float MODEL_RANGE = 32.0f;
    private static final float MODEL_SCALE_INV = POSITION_MAX_VALUE / MODEL_RANGE;

    public static int nvidiumEncodePosition(float v) {
        return (int) ((MODEL_ORIGIN + v) * MODEL_SCALE_INV);
    }

    public static int nvidiumCompactLight(int light) {
        int sky = clamp((light >>> 16) & 0xFF, 8, 248);
        int block = clamp((light >>> 0) & 0xFF, 8, 248);
        return (block << 0) | (sky << 8);
    }

    public static int nvidiumEncodeTexture(float u, float v) {
        return ((Math.round(u * TEXTURE_MAX_VALUE) & 0xFFFF) << 0) |
                ((Math.round(v * TEXTURE_MAX_VALUE) & 0xFFFF) << 16);
    }

    private static int clamp(int val, int min, int max) {
        return val < min ? min : Math.min(val, max);
    }

    // --- Algoritmi Distant Horizons (da DhSectionPos.java) ---
    public static final int DETAIL_LEVEL_WIDTH = 8;
    public static final int X_POS_WIDTH = 28;
    public static final int Z_POS_WIDTH = 28;
    public static final int X_POS_MISSING_WIDTH = 32 - 28;
    public static final int Z_POS_MISSING_WIDTH = 32 - 28;

    public static final int DETAIL_LEVEL_OFFSET = 0;
    public static final int POS_X_OFFSET = DETAIL_LEVEL_OFFSET + DETAIL_LEVEL_WIDTH;
    public static final int POS_Z_OFFSET = POS_X_OFFSET + X_POS_WIDTH;

    public static final long DETAIL_LEVEL_MASK = Byte.MAX_VALUE;
    public static final int POS_X_MASK = (1 << X_POS_WIDTH) - 1;
    public static final int POS_Z_MASK = (1 << Z_POS_WIDTH) - 1;

    public static long dhEncodeSectionPos(byte detailLevel, int x, int z) {
        long data = 0;
        data |= detailLevel & DETAIL_LEVEL_MASK;
        data |= (long) (x & POS_X_MASK) << POS_X_OFFSET;
        data |= (long) (z & POS_Z_MASK) << POS_Z_OFFSET;
        return data;
    }

    public static byte dhGetDetailLevel(long pos) {
        return (byte) ((pos >> DETAIL_LEVEL_OFFSET) & DETAIL_LEVEL_MASK);
    }

    public static int dhGetX(long pos) {
        int x = (int) ((pos >> POS_X_OFFSET) & POS_X_MASK);
        x = (x << X_POS_MISSING_WIDTH) >> X_POS_MISSING_WIDTH;
        return x;
    }

    public static int dhGetZ(long pos) {
        int z = (int) ((pos >> POS_Z_OFFSET) & POS_Z_MASK);
        z = (z << Z_POS_MISSING_WIDTH) >> Z_POS_MISSING_WIDTH;
        return z;
    }

    public static long dhConvertToDetailLevel(long pos, byte newDetailLevel) {
        byte detailLevel = dhGetDetailLevel(pos);
        int x = dhGetX(pos);
        int z = dhGetZ(pos);

        if (newDetailLevel >= detailLevel) {
            int shift = newDetailLevel - detailLevel;
            int divisor = 1 << shift;
            x = Math.floorDiv(x, divisor);
            z = Math.floorDiv(z, divisor);
        } else {
            int shift = detailLevel - newDetailLevel;
            int multiplier = 1 << shift;
            x = x * multiplier;
            z = z * multiplier;
        }

        return dhEncodeSectionPos(newDetailLevel, x, z);
    }

    // --- Networking VarInt (da Minecraft FriendlyByteBuf) ---
    public static byte[] writeVarInt(int value) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        while ((value & -128) != 0) {
            out.write(value & 127 | 128);
            value >>>= 7;
        }
        out.write(value);
        return out.toByteArray();
    }

    public static void main(String[] args) throws IOException {
        String baseDir = args.length > 0 ? args[0] : "testdata";
        File dir = new File(baseDir);
        new File(dir, "nvidium").mkdirs();
        new File(dir, "distant-horizons").mkdirs();
        new File(dir, "network").mkdirs();

        generateNvidiumPositionVectors(new File(dir, "nvidium/encode_position.json"));
        generateNvidiumLightVectors(new File(dir, "nvidium/compact_light.json"));
        generateNvidiumTextureVectors(new File(dir, "nvidium/encode_texture.json"));
        generateDhSectionPosVectors(new File(dir, "distant-horizons/section_pos_encode.json"));
        generateDhConvertVectors(new File(dir, "distant-horizons/section_pos_convert.json"));
        generateVarIntVectors(new File(dir, "network/varint.json"));
        generateNvidiumConfigGolden(new File(dir, "nvidium/config_golden.json"));

        System.out.println("Test vectors generati con successo in: " + dir.getAbsolutePath());
    }

    private static void generateNvidiumPositionVectors(File outFile) throws IOException {
        Random rng = new Random(SEED);
        List<String> cases = new ArrayList<>();

        // Edge cases
        float[] edgeCases = {
            0.0f, -8.0f, 8.0f, 24.0f, -16.0f, 0.5f, -0.5f, 15.999f, -7.999f
        };
        for (float v : edgeCases) {
            cases.add(String.format("{\"input\": {\"v\": %s}, \"expected\": %d}", floatToJson(v), nvidiumEncodePosition(v)));
        }

        // Random cases
        for (int i = 0; i < 50; i++) {
            float v = -8.0f + rng.nextFloat() * 32.0f;
            cases.add(String.format("{\"input\": {\"v\": %s}, \"expected\": %d}", floatToJson(v), nvidiumEncodePosition(v)));
        }

        writeJson(outFile, "nvidium::encode_position", cases);
    }

    private static void generateNvidiumLightVectors(File outFile) throws IOException {
        Random rng = new Random(SEED + 1);
        List<String> cases = new ArrayList<>();

        // Edge cases
        int[] edgeCases = {
            0, 0xFF, 0xFF0000, 0x00FF00, 0xFFFFFF, 0x080008, 0xF800F8, 0x040004, 0xFC00FC
        };
        for (int l : edgeCases) {
            cases.add(String.format("{\"input\": {\"light\": %d}, \"expected\": %d}", l, nvidiumCompactLight(l)));
        }

        // Random cases
        for (int i = 0; i < 50; i++) {
            int l = rng.nextInt();
            cases.add(String.format("{\"input\": {\"light\": %d}, \"expected\": %d}", l, nvidiumCompactLight(l)));
        }

        writeJson(outFile, "nvidium::compact_light", cases);
    }

    private static void generateNvidiumTextureVectors(File outFile) throws IOException {
        Random rng = new Random(SEED + 2);
        List<String> cases = new ArrayList<>();

        // Edge cases
        float[][] edgeCases = {
            {0.0f, 0.0f}, {1.0f, 1.0f}, {0.5f, 0.5f}, {0.25f, 0.75f}, {0.12345f, 0.6789f}
        };
        for (float[] uv : edgeCases) {
            cases.add(String.format("{\"input\": {\"u\": %s, \"v\": %s}, \"expected\": %d}",
                floatToJson(uv[0]), floatToJson(uv[1]), nvidiumEncodeTexture(uv[0], uv[1])));
        }

        // Random cases
        for (int i = 0; i < 50; i++) {
            float u = rng.nextFloat();
            float v = rng.nextFloat();
            cases.add(String.format("{\"input\": {\"u\": %s, \"v\": %s}, \"expected\": %d}",
                floatToJson(u), floatToJson(v), nvidiumEncodeTexture(u, v)));
        }

        writeJson(outFile, "nvidium::encode_texture", cases);
    }

    private static void generateDhSectionPosVectors(File outFile) throws IOException {
        Random rng = new Random(SEED + 3);
        List<String> cases = new ArrayList<>();

        // Edge cases
        int[][] edgeCases = {
            {0, 0, 0}, {6, 0, 0}, {6, 1, 1}, {6, -1, -1}, {6, 1000, -1000},
            {12, 134217727, -134217728}, {6, -500, 500}
        };
        for (int[] pos : edgeCases) {
            byte level = (byte) pos[0];
            int x = pos[1];
            int z = pos[2];
            long packed = dhEncodeSectionPos(level, x, z);
            byte decLevel = dhGetDetailLevel(packed);
            int decX = dhGetX(packed);
            int decZ = dhGetZ(packed);

            cases.add(String.format(
                "{\"input\": {\"level\": %d, \"x\": %d, \"z\": %d}, \"packed\": %d, \"decoded\": {\"level\": %d, \"x\": %d, \"z\": %d}}",
                level, x, z, packed, decLevel, decX, decZ));
        }

        // Random cases
        for (int i = 0; i < 50; i++) {
            byte level = (byte) (rng.nextInt(16));
            // 28-bit signed integer range: -134217728 to 134217727
            int x = (rng.nextInt(1 << 27)) - (1 << 26);
            int z = (rng.nextInt(1 << 27)) - (1 << 26);
            long packed = dhEncodeSectionPos(level, x, z);
            byte decLevel = dhGetDetailLevel(packed);
            int decX = dhGetX(packed);
            int decZ = dhGetZ(packed);

            cases.add(String.format(
                "{\"input\": {\"level\": %d, \"x\": %d, \"z\": %d}, \"packed\": %d, \"decoded\": {\"level\": %d, \"x\": %d, \"z\": %d}}",
                level, x, z, packed, decLevel, decX, decZ));
        }

        writeJson(outFile, "distant_horizons::section_pos_encode", cases);
    }

    private static void generateDhConvertVectors(File outFile) throws IOException {
        Random rng = new Random(SEED + 4);
        List<String> cases = new ArrayList<>();

        // Test conversions across levels
        byte[][] levels = {
            {6, 7}, {6, 8}, {8, 6}, {7, 6}, {6, 10}, {10, 6}
        };
        int[][] coords = {
            {0, 0}, {16, 32}, {-16, -32}, {-1, -1}, {105, -205}
        };

        for (byte[] pair : levels) {
            byte fromLevel = pair[0];
            byte toLevel = pair[1];
            for (int[] c : coords) {
                long pos = dhEncodeSectionPos(fromLevel, c[0], c[1]);
                long converted = dhConvertToDetailLevel(pos, toLevel);
                byte resLevel = dhGetDetailLevel(converted);
                int resX = dhGetX(converted);
                int resZ = dhGetZ(converted);

                cases.add(String.format(
                    "{\"input\": {\"from_level\": %d, \"to_level\": %d, \"x\": %d, \"z\": %d}, \"expected_packed\": %d, \"expected\": {\"level\": %d, \"x\": %d, \"z\": %d}}",
                    fromLevel, toLevel, c[0], c[1], converted, resLevel, resX, resZ));
            }
        }

        writeJson(outFile, "distant_horizons::section_pos_convert", cases);
    }

    private static void generateVarIntVectors(File outFile) throws IOException {
        int[] testValues = {
            0, 1, 2, 127, 128, 255, 25565, 2097151, 2147483647, -1, -2147483648
        };
        List<String> cases = new ArrayList<>();
        for (int v : testValues) {
            byte[] bytes = writeVarInt(v);
            StringBuilder sb = new StringBuilder("[");
            for (int i = 0; i < bytes.length; i++) {
                sb.append(bytes[i] & 0xFF);
                if (i + 1 < bytes.length) sb.append(", ");
            }
            sb.append("]");
            cases.add(String.format("{\"value\": %d, \"bytes\": %s}", v, sb.toString()));
        }

        writeJson(outFile, "network::varint", cases);
    }

    private static void generateNvidiumConfigGolden(File outFile) throws IOException {
        String json = "{\n" +
            "  \"region_keep_distance\": 32,\n" +
            "  \"async_occlusion_culling\": true,\n" +
            "  \"translucent_sorting\": true,\n" +
            "  \"max_draw_distance\": 64,\n" +
            "  \"enable_hiz_culling\": true\n" +
            "}\n";
        try (FileWriter fw = new FileWriter(outFile)) {
            fw.write(json);
        }
    }

    private static String floatToJson(float f) {
        if (Float.isNaN(f)) return "\"NaN\"";
        if (Float.isInfinite(f)) return f > 0 ? "\"Infinity\"" : "\"-Infinity\"";
        return Float.toString(f);
    }

    private static void writeJson(File file, String name, List<String> cases) throws IOException {
        StringBuilder sb = new StringBuilder();
        sb.append("{\n");
        sb.append("  \"function\": \"").append(name).append("\",\n");
        sb.append("  \"cases\": [\n");
        for (int i = 0; i < cases.size(); i++) {
            sb.append("    ").append(cases.get(i));
            if (i + 1 < cases.size()) sb.append(",");
            sb.append("\n");
        }
        sb.append("  ]\n");
        sb.append("}\n");

        try (FileWriter fw = new FileWriter(file)) {
            fw.write(sb.toString());
        }
    }
}

