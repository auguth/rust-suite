// SPDX-License-Identifier: MPL-2.0
//
// Part of Auguth Labs open-source softwares.
// Built for the Rust Programming Language Ecosystem.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2026 Auguth Labs (OPC) Pvt Ltd, India

// ===============================================================================
// `````````````````````````````` IDENTS COLLECTIONS `````````````````````````````
// ===============================================================================

//! Type-level identifier and counter collections.
//!
//! This module provides compile-time utilities for hashing identifiers,
//! validating counter metadata and lineage, constructing identifier
//! collections, expanding historical collections, and deriving collection
//! cardinalities from counter tuples.
//!
//! The utilities are primarily used by the instances type-level machinery and
//! its generated code. They are not intended as a general-purpose collection
//! or identifier API for unrelated crate code.
//!
//! Their public visibility exists to allow the generated instances machinery
//! to use these utilities across the required Rust visibility boundaries.
//! Public visibility does not make these interfaces part of the stable API.
//!
//! The signatures of these utilities may therefore be changed as the
//! instances system evolves. Code outside the instances machinery should not
//! depend on their names, generic parameter ordering, associated types,
//! function signatures, or other implementation details.
//!
//! The module provides utilities for:
//!
//! - `hash_ident` - Computes deterministic compile-time identifier hashes.
//! - Counter metadata validation - Validates counter generic indexes and
//!   counter lineage consistency.
//! - Flat identifier collections - Builds and extends compile-time identifier
//!   and identifier-hash collections.
//! - Historical collections - Expands one- through four-dimensional historical
//!   collections while preserving their existing values.
//! - Counter collection lengths - Derives collection cardinalities from
//!   type-level counter tuples.

// ===============================================================================
// ```````````````````````````````````` IMPORT ```````````````````````````````````
// ===============================================================================

// --- Local Crate Re-Exports ---
use crate::*;

// ===============================================================================
// `````````````````````````````````` HASH BYTES `````````````````````````````````
// ===============================================================================

/// Computes a deterministic 64-bit FNV-1a hash of the given byte sequence.
///
/// The hash is evaluated entirely at compile time when called from a
/// constant context, making it suitable for generating stable identifiers
/// from byte-based names or other static input.
pub const fn hash_ident(bytes: &[u8]) -> u64 {
    // 64-bit FNV-1a
    let mut hash: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }

    hash
}

// ===============================================================================
// ```````````````````````````` COUNTER META-CHECKERS ````````````````````````````
// ===============================================================================

/// Validates that implementation-side counter generic indexes match the
/// corresponding indexes reflected by the trait metadata.
///
/// The `trait_meta` slice contains the trait-side metadata entries, where each
/// entry stores a generic index together with its generic name. The
/// `impl_indexes` slice contains the generic indexes extracted from the
/// implementation-side counter arguments.
///
/// Validation succeeds only when:
///
/// - both slices contain the same number of entries, and
/// - each trait-side generic index matches the implementation-side index at
///   the same position.
///
/// The generic names stored in `trait_meta` are intentionally ignored; this
/// function validates only the structural relationship between the counter
/// positions represented by the two metadata sources.
///
/// This function is `const` so that the generated implementation-side
/// validation checkpoint can evaluate the comparison during compilation.
///
/// For example:
///
/// ```ignore
/// const TRAIT_META: &[(usize, &'static str)] = &[
///     (0, "A"),
///     (2, "C"),
/// ];
///
/// const IMPL_INDEXES: &[usize] = &[0, 2];
///
/// assert!(counter_generics_checker(
///     TRAIT_META,
///     IMPL_INDEXES,
/// ));
/// ```
///
/// A mismatch in either the number or ordering of indexes causes validation
/// to fail:
///
/// ```ignore
/// const TRAIT_META: &[(usize, &'static str)] = &[
///     (0, "A"),
///     (2, "C"),
/// ];
///
/// const IMPL_INDEXES: &[usize] = &[0, 1];
///
/// assert!(!counter_generics_checker(
///     TRAIT_META,
///     IMPL_INDEXES,
/// ));
/// ```
///
/// This provides the final structural comparison  after the
/// implementation-side metadata has been reconstructed. The surrounding
/// generated checker is responsible for comparing the metadata length
/// before invoking this function and for producing the appropriate diagnostic
/// when validation fails.
pub const fn counter_generics_checker(
    trait_meta: &[(usize, &'static str)],
    impl_indexes: &[usize],
) -> bool {
    if trait_meta.len() != impl_indexes.len() {
        return false;
    }

    let mut i = 0;
    while i < trait_meta.len() {
        if trait_meta[i].0 != impl_indexes[i] {
            return false;
        }
        i += 1;
    }

    true
}

/// Checks that active counter dimensions preserve their parent identifier
/// hashes across the current instance and its back affiliate.
///
/// For every active dimension, all preceding dimensions must have matching
/// identifier hashes in the current and back collections.
///
/// A counter value of `0` marks an inactive dimension, so no consistency
/// check is required for that dimension.
///
/// For example:
///
/// ```text
/// Current:
/// [H(crypto), H(sha), H(512)]
///
/// Back:
/// [H(crypto), H(sha), H(256)]
///
/// counters:
/// [1, 1, 2]
/// ```
///
/// The third dimension is active, so the preceding dimensions are checked:
///
/// ```text
/// H(crypto) == H(crypto)
/// H(sha)    == H(sha)
/// ```
///
/// The hashes of the active dimension itself are not compared.
///
/// If any required parent hash differs, the function returns `false`.
/// Otherwise, it returns `true`.
///
/// This function is `const` so counter-lineage consistency can be validated
/// during compile-time evaluation.
pub const fn counters_hash_consistency_checker(
    counters: &[usize],
    current_hashes: &[u64],
    back_counter_hashes: &[u64],
) -> bool {
    let mut i = counters.len();

    while i > 0 {
        i -= 1;

        // inactive dimension
        if counters[i] == 0 {
            continue;
        }

        // dimensions before i must match
        let mut p = 0;

        while p < i {
            if current_hashes[p] != back_counter_hashes[p] {
                return false;
            }

            p += 1;
        }
    }

    true
}

/// Finds the position of a hash within a hash collection.
///
/// Returns the index of the first element equal to `a`, or `None` when the
/// hash does not occur in `hashes`.
///
/// For example:
///
/// ```text
/// hashes = [H(A), H(B), H(C)]
/// a      = H(B)
///
/// result = Some(1)
/// ```
///
/// The search preserves the collection's original ordering and returns the
/// first matching position when duplicate hashes are present.
pub fn ident_check(hashes: &[u64], a: &u64) -> Option<usize> {
    hashes.iter().position(|x| x == a)
}

// ===============================================================================
// ``````````````````````````````` FLAT COLLECTIONS ``````````````````````````````
// ===============================================================================

/// Appends the current identifier to an existing identifier collection.
///
/// The input collection represents the identifier history of the predecessor
/// instance, while `current` represents the identifier introduced by the
/// current instance. The resulting collection therefore contains the complete
/// lineage up to and including the current identifier.
///
/// The collection size must grow by exactly one element:
///
/// ```text
/// N = O + 1
/// ```
///
/// The existing elements are copied in their original order and `current` is
/// written into the final position.
///
/// For example:
///
/// ```text
/// Back:
///     [crypto, image]
///
/// Current:
///     audio
///
/// Result:
///     [crypto, image, audio]
/// ```
///
/// The function is `const` so that identifier collections can be constructed
/// during compile-time evaluation. [`MaybeUninit`] is used internally because
/// the output [`GenericArray`] must be initialized element by element before
/// it can be converted into its fully initialized form.
///
/// This function does not perform identifier uniqueness validation. When
/// uniqueness must be enforced, use
/// [`counter_ident_generic_hash_collection_append`] instead.
pub const fn counter_ident_generic_collection_append<O: ArrayLength, N: ArrayLength>(
    back: &GenericArray<&'static [u8], O>,
    current: &'static [u8],
) -> GenericArray<&'static [u8], N> {
    assert!(N::USIZE == O::USIZE + 1);

    let mut out: GenericArray<MaybeUninit<&'static [u8]>, N> = GenericArray::uninit();

    let src = back.as_slice();
    let dst = out.as_mut_slice();

    let mut i = 0;

    while i < O::USIZE {
        dst[i].write(src[i]);
        i += 1;
    }

    dst[i].write(current);

    unsafe { GenericArray::assume_init(out) }
}

/// Appends the current identifier hash to an existing hash collection while
/// enforcing identifier uniqueness.
///
/// The input collection represents the identifier hashes accumulated by the
/// predecessor instance, while `current` represents the hash of the identifier
/// introduced by the current instance.
///
/// Before appending, the function checks whether `current` already exists in
/// the predecessor collection. If it does, `None` is returned to signal a
/// duplicate identifier. Otherwise, the predecessor collection is copied and
/// `current` is appended as the final element.
///
/// The collection size must grow by exactly one element:
///
/// ```text
/// N = O + 1
/// ```
///
/// For example:
///
/// ```text
/// Back:
///     [H(crypto), H(image)]
///
/// Current:
///     H(audio)
///
/// Result:
///     Some([H(crypto), H(image), H(audio)])
/// ```
///
/// If the current identifier has already occurred in the lineage:
///
/// ```text
/// Back:
///     [H(crypto), H(image)]
///
/// Current:
///     H(image)
///
/// Result:
///     None
/// ```
///
/// The function is `const` so that both the uniqueness check and collection
/// construction can be evaluated during compile-time processing.
///
/// [`MaybeUninit`] is used to construct the output [`GenericArray`] element by
/// element before it is converted into its initialized representation.
///
/// This helper is used by the implementation-side hash collection to detect
/// duplicate identifiers within a counter lineage. The surrounding generated
/// expression converts `None` into the appropriate compile-time diagnostic.
pub const fn counter_ident_generic_hash_collection_append<O: ArrayLength, N: ArrayLength>(
    back: &GenericArray<u64, O>,
    current: u64,
) -> Option<GenericArray<u64, N>> {
    assert!(N::USIZE == O::USIZE + 1);

    let src = back.as_slice();

    let mut i = 0;
    while i < O::USIZE {
        if src[i] == current {
            return None;
        }
        i += 1;
    }

    let mut out: GenericArray<MaybeUninit<u64>, N> = GenericArray::uninit();

    let dst = out.as_mut_slice();

    let mut i = 0;
    while i < O::USIZE {
        dst[i].write(src[i]);
        i += 1;
    }

    dst[i].write(current);

    Some(unsafe { GenericArray::assume_init(out) })
}

// ===============================================================================
// ````````````````````````` HISTORICAL (N-D) COLLECTIONS ````````````````````````
// ===============================================================================

/// Expands a one-dimensional historical collection to the new cardinality
/// and records a value at the specified coordinate.
///
/// Existing entries are preserved in their original positions. Any newly
/// introduced positions are initialized to `None`, after which `value` is
/// written at `index`.
///
/// The output cardinality must be at least the input cardinality, and
/// `index` must lie within the output collection:
///
/// ```text
/// N >= O
/// index < N
/// ```
///
/// For example:
///
/// ```text
/// Old:
/// [Some(A), Some(B)]
///
/// New:
/// [Some(A), Some(B), None, None]
///
/// index = 2
/// value = Some(C)
///
/// Result:
/// [Some(A), Some(B), Some(C), None]
/// ```
///
/// This function is `const` so historical collection expansion can be
/// evaluated during compile-time processing.
pub const fn historical_collection_1<T: Copy, O: ArrayLength, N: ArrayLength>(
    old: &GenericArray<Option<T>, O>,
    value: Option<T>,
    index: usize,
) -> GenericArray<Option<T>, N> {
    assert!(index < N::USIZE);
    assert!(N::USIZE >= O::USIZE);

    let src = old.as_slice();

    let mut out: GenericArray<MaybeUninit<Option<T>>, N> = GenericArray::uninit();

    let dst = out.as_mut_slice();

    let mut i = 0;
    while i < N::USIZE {
        dst[i].write(if i < O::USIZE { src[i] } else { None });

        i += 1;
    }

    dst[index].write(value);

    unsafe { GenericArray::assume_init(out) }
}

/// Expands a two-dimensional historical collection to the new cardinalities
/// and records a value at the specified two-dimensional coordinate.
///
/// Existing entries are preserved at their original coordinates. Coordinates
/// introduced by either dimension are initialized to `None`, after which
/// `value` is written at `(index_a, index_b)`.
///
/// Both output dimensions must be at least as large as their corresponding
/// input dimensions:
///
/// ```text
/// NA >= OA
/// NB >= OB
///
/// index_a < NA
/// index_b < NB
/// ```
///
/// For example:
///
/// ```text
/// Old:
/// [
///     [Some(A), Some(B)],
///     [Some(C), None],
/// ]
///
/// Expanded:
/// [
///     [Some(A), Some(B), None],
///     [Some(C), None,    None],
/// ]
///
/// (index_a, index_b) = (1, 1)
/// value = Some(D)
///
/// Result:
/// [
///     [Some(A), Some(B), None],
///     [Some(C), Some(D), None],
/// ]
/// ```
///
/// This function is `const` so historical collection expansion can be
/// evaluated during compile-time processing.
pub const fn historical_collection_2<
    T: Copy,
    OA: ArrayLength,
    OB: ArrayLength,
    NA: ArrayLength,
    NB: ArrayLength,
>(
    old: &GenericArray<GenericArray<Option<T>, OB>, OA>,
    value: Option<T>,
    index_a: usize,
    index_b: usize,
) -> GenericArray<GenericArray<Option<T>, NB>, NA> {
    assert!(index_a < NA::USIZE);
    assert!(index_b < NB::USIZE);

    assert!(NA::USIZE >= OA::USIZE);
    assert!(NB::USIZE >= OB::USIZE);

    let src = old.as_slice();

    let mut out: GenericArray<MaybeUninit<GenericArray<Option<T>, NB>>, NA> =
        GenericArray::uninit();

    let dst = out.as_mut_slice();

    let mut a = 0;
    while a < NA::USIZE {
        let mut row: GenericArray<MaybeUninit<Option<T>>, NB> = GenericArray::uninit();

        let row_dst = row.as_mut_slice();

        let mut b = 0;
        while b < NB::USIZE {
            row_dst[b].write(if a < OA::USIZE && b < OB::USIZE {
                src[a].as_slice()[b]
            } else {
                None
            });

            b += 1;
        }

        if a == index_a {
            row_dst[index_b].write(value);
        }

        dst[a].write(unsafe { GenericArray::assume_init(row) });

        a += 1;
    }

    unsafe { GenericArray::assume_init(out) }
}

/// Expands a three-dimensional historical collection to the new cardinalities
/// and records a value at the specified three-dimensional coordinate.
///
/// Existing entries are preserved at their original coordinates. Coordinates
/// introduced by any dimension are initialized to `None`, after which `value`
/// is written at `(ia, ib, ic)`.
///
/// The output dimensions must not be smaller than their corresponding input
/// dimensions, and every supplied coordinate must lie within the output shape:
///
/// ```text
/// NA >= OA
/// NB >= OB
/// NC >= OC
///
/// ia < NA
/// ib < NB
/// ic < NC
/// ```
///
/// For example:
///
/// ```text
/// Old:
/// [
///     [
///         [Some(A), Some(B)],
///         [Some(C), None],
///     ],
/// ]
///
/// Expanded:
/// [
///     [
///         [Some(A), Some(B), None],
///         [Some(C), None,    None],
///     ],
/// ]
///
/// (ia, ib, ic) = (0, 1, 1)
/// value = Some(D)
///
/// Result:
/// [
///     [
///         [Some(A), Some(B), None],
///         [Some(C), Some(D), None],
///     ],
/// ]
/// ```
///
/// This function is `const` so historical collection expansion can be
/// evaluated during compile-time processing.
pub const fn historical_collection_3<
    T: Copy,
    OA: ArrayLength,
    OB: ArrayLength,
    OC: ArrayLength,
    NA: ArrayLength,
    NB: ArrayLength,
    NC: ArrayLength,
>(
    old: &GenericArray<GenericArray<GenericArray<Option<T>, OC>, OB>, OA>,
    value: Option<T>,
    ia: usize,
    ib: usize,
    ic: usize,
) -> GenericArray<GenericArray<GenericArray<Option<T>, NC>, NB>, NA> {
    assert!(ia < NA::USIZE);
    assert!(ib < NB::USIZE);
    assert!(ic < NC::USIZE);

    assert!(NA::USIZE >= OA::USIZE);
    assert!(NB::USIZE >= OB::USIZE);
    assert!(NC::USIZE >= OC::USIZE);

    let src = old.as_slice();

    let mut out: GenericArray<MaybeUninit<GenericArray<GenericArray<Option<T>, NC>, NB>>, NA> =
        GenericArray::uninit();

    let dst_a = out.as_mut_slice();

    let mut a = 0;
    while a < NA::USIZE {
        let mut plane: GenericArray<MaybeUninit<GenericArray<Option<T>, NC>>, NB> =
            GenericArray::uninit();

        let dst_b = plane.as_mut_slice();

        let mut b = 0;
        while b < NB::USIZE {
            let mut row: GenericArray<MaybeUninit<Option<T>>, NC> = GenericArray::uninit();

            let dst_c = row.as_mut_slice();

            let mut c = 0;
            while c < NC::USIZE {
                dst_c[c].write(if a < OA::USIZE && b < OB::USIZE && c < OC::USIZE {
                    src[a].as_slice()[b].as_slice()[c]
                } else {
                    None
                });

                c += 1;
            }

            if a == ia && b == ib {
                dst_c[ic].write(value);
            }

            dst_b[b].write(unsafe { GenericArray::assume_init(row) });

            b += 1;
        }

        dst_a[a].write(unsafe { GenericArray::assume_init(plane) });

        a += 1;
    }

    unsafe { GenericArray::assume_init(out) }
}

/// Expands a four-dimensional historical collection to the new cardinalities
/// and records a value at the specified four-dimensional coordinate.
///
/// Existing entries are preserved at their original coordinates. Coordinates
/// introduced by any dimension are initialized to `None`, after which `value`
/// is written at `(ia, ib, ic, id)`.
///
/// The output dimensions must not be smaller than their corresponding input
/// dimensions, and every supplied coordinate must lie within the output shape:
///
/// ```text
/// NA >= OA
/// NB >= OB
/// NC >= OC
/// ND >= OD
///
/// ia < NA
/// ib < NB
/// ic < NC
/// id < ND
/// ```
///
/// For example:
///
/// ```text
/// Old:
/// [
///     [
///         [
///             [Some(A), Some(B)],
///             [Some(C), None],
///         ],
///     ],
/// ]
///
/// Expanded:
/// [
///     [
///         [
///             [Some(A), Some(B), None],
///             [Some(C), None,    None],
///         ],
///     ],
/// ]
///
/// (ia, ib, ic, id) = (0, 0, 1, 1)
/// value = Some(D)
///
/// Result:
/// [
///     [
///         [
///             [Some(A), Some(B), None],
///             [Some(C), Some(D), None],
///         ],
///     ],
/// ]
/// ```
///
/// This function is `const` so historical collection expansion can be
/// evaluated during compile-time processing.
pub const fn historical_collection_4<
    T: Copy,
    OA: ArrayLength,
    OB: ArrayLength,
    OC: ArrayLength,
    OD: ArrayLength,
    NA: ArrayLength,
    NB: ArrayLength,
    NC: ArrayLength,
    ND: ArrayLength,
>(
    old: &GenericArray<GenericArray<GenericArray<GenericArray<Option<T>, OD>, OC>, OB>, OA>,
    value: Option<T>,
    ia: usize,
    ib: usize,
    ic: usize,
    id: usize,
) -> GenericArray<GenericArray<GenericArray<GenericArray<Option<T>, ND>, NC>, NB>, NA> {
    assert!(ia < NA::USIZE);
    assert!(ib < NB::USIZE);
    assert!(ic < NC::USIZE);
    assert!(id < ND::USIZE);

    let src = old.as_slice();

    let mut out: GenericArray<
        MaybeUninit<GenericArray<GenericArray<GenericArray<Option<T>, ND>, NC>, NB>>,
        NA,
    > = GenericArray::uninit();

    let dst_a = out.as_mut_slice();

    let mut a = 0;
    while a < NA::USIZE {
        let mut cube: GenericArray<MaybeUninit<GenericArray<GenericArray<Option<T>, ND>, NC>>, NB> =
            GenericArray::uninit();

        let dst_b = cube.as_mut_slice();

        let mut b = 0;
        while b < NB::USIZE {
            let mut plane: GenericArray<MaybeUninit<GenericArray<Option<T>, ND>>, NC> =
                GenericArray::uninit();

            let dst_c = plane.as_mut_slice();

            let mut c = 0;
            while c < NC::USIZE {
                let mut row: GenericArray<MaybeUninit<Option<T>>, ND> = GenericArray::uninit();

                let dst_d = row.as_mut_slice();

                let mut d = 0;
                while d < ND::USIZE {
                    dst_d[d].write(
                        if a < OA::USIZE && b < OB::USIZE && c < OC::USIZE && d < OD::USIZE {
                            src[a].as_slice()[b].as_slice()[c].as_slice()[d]
                        } else {
                            None
                        },
                    );

                    d += 1;
                }

                if a == ia && b == ib && c == ic {
                    dst_d[id].write(value);
                }

                dst_c[c].write(unsafe { GenericArray::assume_init(row) });

                c += 1;
            }

            dst_b[b].write(unsafe { GenericArray::assume_init(plane) });

            b += 1;
        }

        dst_a[a].write(unsafe { GenericArray::assume_init(cube) });

        a += 1;
    }

    unsafe { GenericArray::assume_init(out) }
}

// ===============================================================================
// ``````````````````````````` COUNTERS COLLECTION LEN ```````````````````````````
// ===============================================================================

/// Derives the collection length for one dimension from a tuple of counters.
///
/// `Counters` represents the counters of all dimensions, with each counter
/// stored at the position corresponding to its dimension:
///
/// ```text
/// Counters = (U0, U1, U2)
///             |   |   |
///             |   |   |--- dimension 2 counter
///             |   |------- dimension 1 counter
///             |----------- dimension 0 counter
/// ```
///
/// `Index` selects one counter from this tuple. The selected counter
/// represents the last zero-based position occupied in that dimension.
/// Therefore, its collection length is one greater than the counter:
///
/// ```text
/// selected counter = N
/// collection length = N + 1
/// ```
///
/// For example, given:
///
/// ```text
/// Counters = (U0, U1, U2)
/// ```
///
/// the collection length for each dimension is:
///
/// ```text
/// Index 0:
///     counter = U0
///     length  = U0 + U1 = U1
///
/// Index 1:
///     counter = U1
///     length  = U1 + U1 = U2
///
/// Index 2:
///     counter = U2
///     length  = U2 + U1 = U3
/// ```
///
/// In other words, `CountersCollectionLen<Index>` performs two type-level
/// operations:
///
/// ```text
/// Counters
///     | CountersGet<Index>
/// counter at Index
///     | + U1
/// collection length
/// ```
///
/// For a larger counter tuple:
///
/// ```text
/// Counters = (U2, U4, U1)
/// ```
///
/// the dimensions have the following collection lengths:
///
/// ```text
/// dimension 0    counter U2    length U3
/// dimension 1    counter U4    length U5
/// dimension 2    counter U1    length U2
/// ```
///
/// The resulting `Output` is a type-level [`Unsigned`] value and can
/// therefore be used as the cardinality of a compile-time collection such
/// as `GenericArray`.
pub trait CountersCollectionLen<Index: UnsignedTypeNum> {
    type Output: UnsignedTypeNum;
}

impl<Counters, Index, Counter> CountersCollectionLen<Index> for Counters
where
    Index: UnsignedTypeNum,
    Counters: CountersGet<Index, Output = Counter>,
    Counter: UnsignedTypeNum + Add<U1>,
    Sum<Counter, U1>: UnsignedTypeNum,
{
    type Output = Sum<Counter, U1>;
}
