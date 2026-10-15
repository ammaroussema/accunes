// web port video output!!!

export class NesRenderer {
    constructor(canvas) {
        this.canvas = canvas;
        this.gl = null;
        this.ctx2d = null;
        this.texture = null;
        this.program = null;
        this.filterMode = 'pixelated'; 
        this.aspectRatio = '4:3';
        this.init();
    }

    init() {
        const gl = this.canvas.getContext('webgl2', { alpha: false, depth: false, antialias: false });
        if (gl) {
            this.gl = gl;
            this.initWebGL();
        } else {
            console.warn('WebGL2 not supported, falling back to 2D Canvas');
            this.ctx2d = this.canvas.getContext('2d');
            this.imageData = this.ctx2d.createImageData(256, 240);
            this.pixelBuf32 = new Uint32Array(this.imageData.data.buffer);
        }
    }

    initWebGL() {
        const gl = this.gl;

        const vsSource = `#version 300 es
            in vec2 a_pos;
            in vec2 a_uv;
            out vec2 v_uv;
            void main() {
                v_uv = a_uv;
                gl_Position = vec4(a_pos, 0.0, 1.0);
            }
        `;

        const fsSource = `#version 300 es
            precision mediump float;
            in vec2 v_uv;
            out vec4 out_color;
            uniform sampler2D u_texture;
            uniform int u_filter;

            void main() {
                vec4 tex = texture(u_texture, v_uv);
                vec3 color = tex.bgr;

                if (u_filter == 2) {
                    float scanline = sin(v_uv.y * 240.0 * 3.14159265);
                    color *= (0.88 + 0.12 * scanline);

                    vec2 uv_norm = v_uv * 2.0 - 1.0;
                    float vig = 1.0 - dot(uv_norm, uv_norm) * 0.08;
                    color *= clamp(vig, 0.0, 1.0);
                }

                out_color = vec4(color, 1.0);
            }
        `;

        const vs = this.compileShader(gl.VERTEX_SHADER, vsSource);
        const fs = this.compileShader(gl.FRAGMENT_SHADER, fsSource);

        const program = gl.createProgram();
        gl.attachShader(program, vs);
        gl.attachShader(program, fs);
        gl.linkProgram(program);

        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            console.error('Failed to link shader program:', gl.getProgramInfoLog(program));
            return;
        }

        this.program = program;
        gl.useProgram(program);

        const vertices = new Float32Array([
            -1.0, -1.0,    0.0, 1.0,
             1.0, -1.0,    1.0, 1.0,
            -1.0,  1.0,    0.0, 0.0,
            -1.0,  1.0,    0.0, 0.0,
             1.0, -1.0,    1.0, 1.0,
             1.0,  1.0,    1.0, 0.0,
        ]);

        const vao = gl.createVertexArray();
        gl.bindVertexArray(vao);

        const vbo = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
        gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);

        const aPos = gl.getAttribLocation(program, 'a_pos');
        gl.enableVertexAttribArray(aPos);
        gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 16, 0);

        const aUv = gl.getAttribLocation(program, 'a_uv');
        gl.enableVertexAttribArray(aUv);
        gl.vertexAttribPointer(aUv, 2, gl.FLOAT, false, 16, 8);

        this.texture = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, this.texture);
        gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        this.updateTextureFilter();

        gl.texImage2D(
            gl.TEXTURE_2D, 0, gl.RGBA,
            256, 240, 0,
            gl.RGBA, gl.UNSIGNED_BYTE,
            null
        );

        this.uFilter = gl.getUniformLocation(program, 'u_filter');
    }

    compileShader(type, source) {
        const gl = this.gl;
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            console.error('Shader compilation error:', gl.getShaderInfoLog(shader));
            gl.deleteShader(shader);
            return null;
        }
        return shader;
    }

    setFilter(mode) {
        this.filterMode = mode;
        if (this.gl) {
            this.updateTextureFilter();
        }
    }

    updateTextureFilter() {
        const gl = this.gl;
        if (!gl || !this.texture) return;
        gl.bindTexture(gl.TEXTURE_2D, this.texture);
        const filter = (this.filterMode === 'smooth') ? gl.LINEAR : gl.NEAREST;
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
    }

    render(memoryBuffer, ptr) {
        if (this.gl) {
            const gl = this.gl;
            if (!this.byteView || this.byteView.buffer !== memoryBuffer || this.byteView.byteOffset !== ptr) {
                this.byteView = new Uint8Array(memoryBuffer, ptr, 256 * 240 * 4);
            }

            gl.activeTexture(gl.TEXTURE0);
            gl.bindTexture(gl.TEXTURE_2D, this.texture);
            gl.texSubImage2D(
                gl.TEXTURE_2D, 0,
                0, 0,
                256, 240,
                gl.RGBA, gl.UNSIGNED_BYTE,
                this.byteView
            );

            gl.useProgram(this.program);
            const modeInt = this.filterMode === 'crt' ? 2 : (this.filterMode === 'smooth' ? 1 : 0);
            gl.uniform1i(this.uFilter, modeInt);

            gl.viewport(0, 0, this.canvas.width, this.canvas.height);
            gl.drawArrays(gl.TRIANGLES, 0, 6);
        } else if (this.ctx2d) {
            const src32 = new Uint32Array(memoryBuffer, ptr, 256 * 240);
            const dst32 = this.pixelBuf32;
            for (let i = 0; i < 256 * 240; i++) {
                const p = src32[i];
                const b = (p >> 16) & 0xFF;
                const g = (p >> 8) & 0xFF;
                const r = p & 0xFF;
                dst32[i] = 0xFF000000 | (b << 16) | (g << 8) | r;
            }
            this.ctx2d.putImageData(this.imageData, 0, 0);
        }
    }
}
