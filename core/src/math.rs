#[inline(always)]
pub fn q_abs(x: f32) -> f32 {
    f32::from_bits(x.to_bits() & 0x7FFFFFFF)
}

#[inline(always)]
pub fn q_sign(x: f32) -> f32 {
    f32::from_bits((x.to_bits() & 0x80000000) | 0x3F800000)
}

#[inline(always)]
pub fn q_sin(x: f32) -> f32 {
    let pi = std::f32::consts::PI;
    let period = 2.0 * pi;
    let mut x_mod = x - period * (x / period).floor();
    if x_mod > pi {
        x_mod -= period;
    }
    let mut sin_x = (4.0 / pi) * x_mod - (4.0 / (pi * pi)) * x_mod * q_abs(x_mod);
    sin_x = 0.225 * (sin_x * q_abs(sin_x) - sin_x) + sin_x;
    sin_x
}

#[inline(always)]
pub fn q_sin_8x(data: &mut [f32]) {
    let mut i = 0;
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            unsafe {
                use core::arch::x86_64::*;
                // SIMD Constants
                let pi = _mm256_set1_ps(std::f32::consts::PI);
                let period = _mm256_set1_ps(2.0 * std::f32::consts::PI);
                let inv_period = _mm256_set1_ps(1.0 / (2.0 * std::f32::consts::PI));
                let c_4_pi = _mm256_set1_ps(4.0 / std::f32::consts::PI);
                let c_4_pi_sq = _mm256_set1_ps(4.0 / (std::f32::consts::PI * std::f32::consts::PI));
                let c_0_225 = _mm256_set1_ps(0.225);
                let abs_mask = _mm256_castsi256_ps(_mm256_set1_epi32(0x7FFFFFFF_u32 as i32));

                while i + 8 <= data.len() {
                    let x = _mm256_loadu_ps(data.as_ptr().add(i));
                    
                    let div = _mm256_mul_ps(x, inv_period);
                    let floor = _mm256_floor_ps(div);
                    let rem = _mm256_mul_ps(period, floor);
                    let mut x_mod = _mm256_sub_ps(x, rem);
                    
                    let cmp = _mm256_cmp_ps(x_mod, pi, _CMP_GT_OQ);
                    let sub = _mm256_and_ps(cmp, period);
                    x_mod = _mm256_sub_ps(x_mod, sub);
                    
                    let abs_x_mod = _mm256_and_ps(x_mod, abs_mask);
                    
                    let p1 = _mm256_mul_ps(c_4_pi, x_mod);
                    let p2 = _mm256_mul_ps(_mm256_mul_ps(c_4_pi_sq, x_mod), abs_x_mod);
                    let mut sin_x = _mm256_sub_ps(p1, p2);
                    
                    let abs_sin_x = _mm256_and_ps(sin_x, abs_mask);
                    let t1 = _mm256_mul_ps(sin_x, abs_sin_x);
                    let t2 = _mm256_sub_ps(t1, sin_x);
                    let t3 = _mm256_mul_ps(c_0_225, t2);
                    sin_x = _mm256_add_ps(t3, sin_x);
                    
                    _mm256_storeu_ps(data.as_mut_ptr().add(i), sin_x);
                    i += 8;
                }
            }
        }
    }
    
    // Scalar fallback
    while i < data.len() {
        data[i] = q_sin(data[i]);
        i += 1;
    }
}
