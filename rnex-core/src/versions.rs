use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr};
use typenum::{Cmp, IsEqual, IsLess, IsLessOrEqual, Unsigned};

/// This trait represents a version at compile time
trait Version{
    type Major: Unsigned;
    type Minor: Unsigned;
}

/// This struct contains nothing and is used to represent specific versions as an instance of
/// [`Version`]. It is instances as `Ver<Major, Minor>`
struct Ver<MAJ: Unsigned, MIN: Unsigned>{
    _phantom: PhantomData<(MAJ, MIN)>
}

impl<MAJ: Unsigned, MIN: Unsigned> Version for Ver<MAJ, MIN>{
    type Major = MAJ;
    type Minor = MIN;
}

/// Represents two versions which can be compared
trait ComparableVersion<T: Version>: Version{
    type IsAtLeast: SameOrUnit;
}

impl<T: Version, U: Version> ComparableVersion<T> for U where
    <T as Version>::Major: Cmp<Self::Major>,
    <T as Version>::Minor: IsLessOrEqual<Self::Minor>,
    <T as Version>::Major: IsEqual<
        Self::Major,
        Output: BitAnd<
            typenum::LeEq<T::Minor, Self::Minor>
        >,
    >,
    <T as Version>::Major: IsLess<
        Self::Major,
        Output: BitOr<
            typenum::And<
                typenum::Eq<T::Major, Self::Major>,
                typenum::LeEq<T::Minor, Self::Minor>,
            >,
            Output: SameOrUnit
        >
    > {

    type IsAtLeast = typenum::Or<
        typenum::Le<T::Major, Self::Major>,
        typenum::And<
            typenum::Eq<T::Major, Self::Major>,
            typenum::LeEq<T::Minor, Self::Minor>,
        >
    >;
}


/// Simple check for testing if the `TEST` version is at least `REQ` or higher.
type VersionAbove<REQ, TEST> = <TEST as ComparableVersion<REQ>>::IsAtLeast;

trait VersionIsAtLeast<VER: Version>{}

impl<VER: Version, T: ComparableVersion<VER, IsAtLeast = typenum::True>> VersionIsAtLeast<VER> for T{}


/// Trait for containing the result of elements which only conditionally exist
trait CondElemResult{
    type Output;
}

/// Empty helper struct which only servers to give a concrete type when creating fields in rmc
/// structs which have a version requirement. This is not meant to be used directly, use
/// [`MinVersion`] instead.
struct MinVersionElementHelper<T, REQUIRED: Version, VER: Version + ComparableVersion<REQUIRED>>{
    _phantom: PhantomData<(T, REQUIRED, VER)>
}

/// This should be used either with [`typenum::True`] or [`typenum::False`]. When `True` the [`Self::Output`]
/// will be the same as the `T` you put into Output. When `False` it will always be `()`
trait SameOrUnit{
    type Output<T>;
}

impl SameOrUnit for typenum::True{
    type Output<T> = T;
}

impl SameOrUnit for typenum::False{
    type Output<T> = ();
}

impl<T, REQUIRED: Version, VER: Version + ComparableVersion<REQUIRED>> CondElemResult for MinVersionElementHelper<T, REQUIRED, VER> where {
    type Output = <<VER as ComparableVersion<REQUIRED>>::IsAtLeast as SameOrUnit>::Output<T>;
}

/// When the version condition is met the field will exist and will simply be `T` if not it will be
/// replaced by `()`. Use this when you need to add versioning to rmc structs.
type MinVersion<T, REQUIRED, VER> = <MinVersionElementHelper<T, REQUIRED, VER> as CondElemResult>::Output;