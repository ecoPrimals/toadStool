// SPDX-License-Identifier: AGPL-3.0-or-later
//! Embedded PTX kernel sources

pub const MATMUL_SIMPLE: &str = r#"
.version 7.0
.target sm_50
.address_size 64

.visible .entry matmul_simple(
    .param .u64 a,
    .param .u64 b,
    .param .u64 c,
    .param .u32 n
) {
    .reg .u32 %tid, %n_reg;
    .reg .u64 %a_ptr, %b_ptr, %c_ptr;
    .reg .f32 %a_val, %b_val, %c_val;
    
    mov.u32 %tid, %tid.x;
    ld.param.u32 %n_reg, [n];
    
    setp.ge.u32 p, %tid, %n_reg;
    @p bra DONE;
    
    ld.param.u64 %a_ptr, [a];
    ld.param.u64 %b_ptr, [b];
    ld.param.u64 %c_ptr, [c];
    
    .reg .u64 %offset;
    cvt.u64.u32 %offset, %tid;
    shl.b64 %offset, %offset, 2;
    
    add.u64 %a_ptr, %a_ptr, %offset;
    add.u64 %b_ptr, %b_ptr, %offset;
    add.u64 %c_ptr, %c_ptr, %offset;
    
    ld.global.f32 %a_val, [%a_ptr];
    ld.global.f32 %b_val, [%b_ptr];
    mul.f32 %c_val, %a_val, %b_val;
    st.global.f32 [%c_ptr], %c_val;
    
DONE:
    ret;
}
"#;

pub const REDUCE_SUM: &str = r#"
.version 7.0
.target sm_50
.address_size 64

.visible .entry reduce_sum(
    .param .u64 input,
    .param .u64 output,
    .param .u32 n
) {
    .shared .f32 sdata[256];
    .reg .u32 %tid, %n_reg, %bid, %gid;
    .reg .u64 %input_ptr, %output_ptr;
    .reg .f32 %val, %temp;
    
    mov.u32 %tid, %tid.x;
    mov.u32 %bid, %ctaid.x;
    
    .reg .u32 %bsize;
    mov.u32 %bsize, 256;
    mad.lo.u32 %gid, %bid, %bsize, %tid;
    
    ld.param.u32 %n_reg, [n];
    ld.param.u64 %input_ptr, [input];
    
    mov.f32 %val, 0.0;
    setp.lt.u32 p, %gid, %n_reg;
    @!p bra SKIP_LOAD;
    
    .reg .u64 %offset;
    cvt.u64.u32 %offset, %gid;
    shl.b64 %offset, %offset, 2;
    add.u64 %input_ptr, %input_ptr, %offset;
    ld.global.f32 %val, [%input_ptr];
    
SKIP_LOAD:
    st.shared.f32 [sdata + %tid * 4], %val;
    bar.sync 0;
    
    .reg .u32 %s;
    mov.u32 %s, 128;
REDUCE_LOOP:
    setp.ge.u32 p, %tid, %s;
    @p bra SKIP_REDUCE;
    
    .reg .u32 %tid_plus_s;
    add.u32 %tid_plus_s, %tid, %s;
    ld.shared.f32 %temp, [sdata + %tid_plus_s * 4];
    ld.shared.f32 %val, [sdata + %tid * 4];
    add.f32 %val, %val, %temp;
    st.shared.f32 [sdata + %tid * 4], %val;
    
SKIP_REDUCE:
    bar.sync 0;
    shr.u32 %s, %s, 1;
    setp.gt.u32 p, %s, 0;
    @p bra REDUCE_LOOP;
    
    setp.ne.u32 p, %tid, 0;
    @p bra DONE;
    
    ld.param.u64 %output_ptr, [output];
    .reg .u64 %out_offset;
    cvt.u64.u32 %out_offset, %bid;
    shl.b64 %out_offset, %out_offset, 2;
    add.u64 %output_ptr, %output_ptr, %out_offset;
    ld.shared.f32 %val, [sdata];
    st.global.f32 [%output_ptr], %val;
    
DONE:
    ret;
}
"#;
