#version 450

layout(local_size_x = 256) in;

layout(binding = 0) readonly buffer BufferA {
    float data[];
} bufferA;

layout(binding = 1) readonly buffer BufferB {
    float data[];
} bufferB;

layout(binding = 2) writeonly buffer BufferC {
    float data[];
} bufferC;

void main() {
    uint index = gl_GlobalInvocationID.x;
    bufferC.data[index] = bufferA.data[index] + bufferB.data[index];
}

