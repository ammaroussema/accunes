#include <metal_stdlib>
using namespace metal;

struct VertexOutput {
    float4 position [[position]];
    float2 texCoords;
};

vertex VertexOutput vertexShader(uint vertexID [[vertex_id]]) {
    const float2 positions[4] = {
        float2(-1.0,  1.0),
        float2(-1.0, -1.0),
        float2( 1.0,  1.0),
        float2( 1.0, -1.0)
    };

    const float2 texCoords[4] = {
        float2(0.0, 0.0),
        float2(0.0, 1.0),
        float2(1.0, 0.0),
        float2(1.0, 1.0)
    };

    VertexOutput out;
    out.position = float4(positions[vertexID], 0.0, 1.0);
    out.texCoords = texCoords[vertexID];
    return out;
}

fragment float4 fragmentShader(VertexOutput in [[stage_in]],
                               texture2d<float> colorTexture [[texture(0)]],
                               sampler textureSampler [[sampler(0)]]) {
    return colorTexture.sample(textureSampler, in.texCoords);
}
