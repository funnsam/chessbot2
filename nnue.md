# Model
```
  P × 64
  N × 64
  B × 64
  R × 64
  Q × 64
  K × 64
  p × 64
  n × 64
  b × 64
  r × 64
  q × 64
  k × 64
     ↓
[bool; 768] (l0)
→ linear i16×16 (l1) → ReLU
→ linear i16×8  (l2) → ReLU
→ i16 (l3, eval)
```

(768 * 16 + 16) + (16 * 8 + 8) + (8 * 1 + 1)
= 12,449 params

# Forward prop
## AVX-2
```
input (i16×16) → _mm256_madd_epi16 (i32×8)       ┐
         weights (i16×16) ↑  └→ _mm256_add_epi32 ┘ ×8
_mm256_cvtsepi32_epi16 (i16×8) ← acc ←┘
  ↓
_mm_madd_epi16 (i32×4) → normal add → i32 → i16
  ↑
weights (i16×8)
```

## AVX
```
input (i16×8) → _mm_madd_epi16 (i32×4) ┐    ┐
  weights (i16×8) ↑  └→ _mm_add_epi32  ┘ ×8 │ ×2
                        acc ←┘              ┘
_mm_packs_epi32 (i16×8) ←┘
  ↓
_mm_madd_epi16 (i32×4) → normal add → i32 → i16
  ↑
weights (i16×8)
```

## NEON (64-bits)
https://stackoverflow.com/questions/69659665/neon-equivalent-of-mm-madd-epi16-and-mm-maddubs-epi16
```
input (i16×8) → vget_low_s16 (i16×4) → vmull_s16 (i32×4) ┐      ┐
    └→ vmull_high_s16 (i32×4) → vpaddq_s32 ←┘            │ ×8   │
                         vpaddq_s32 ←┘                   ┘      │ ×2
vqmovn_s32 (i16×4) ← acc ←┘                                     │
 └→ vmull_s16 (i32×4) ← weights (i16×4)                         ┘
        └→ normal add → i32 → i16
```

# Backprop
C = cost
a = activation
z = pre activation
y = desired
w = weight
b = bias

```
y
↓
C ← a₃ ← z₃ ← w₃,₀,₀ a₂,₀
         ↑    w₃,₁,₀ a₂,₁
         b₃   …

a₂,₀ ← z₂,₀ ← w₂,₀,₀ a₁,₀
        ↑     w₂,₁,₀ a₁,₁
       b₂,₀   …

a₂,₁ ← z₂,₁ ← w₂,₀,₁ a₁,₀
        ↑     w₂,₁,₁ a₁,₁
       b₂,₁   …      ^^^^- affects multiple paths
```

## Activation a₁,₀
```
 ∂C     i  ∂z₂,ᵢ ∂a₂,ᵢ  ∂C
───── = Σ  ───── ───── ─────
∂a₁,₀   n₂ ∂a₁,₀ ∂z₂,ᵢ ∂a₂,ᵢ

∂z₂,ᵢ
───── = w₂,₀,ᵢ
∂a₁,₀

∂a₂,ᵢ
───── = ReLU'(∂z₂,ᵢ)
∂z₂,ᵢ
```

## Activation a₂,₀
```
 ∂C      ∂z₃  ∂a₃ ∂C
───── = ───── ─── ───
∂a₂,₀   ∂a₂,₀ ∂z₃ ∂a₃

 ∂z₃
───── = w₃,₀,₀
∂a₂,₀

∂a₃
─── = 1
∂z₃

∂C
─── = 2(a₃ - y)
∂a₃
```
