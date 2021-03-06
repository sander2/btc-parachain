use crate::Config;
use frame_support::traits::Currency;
use sp_arithmetic::FixedPointNumber;

pub(crate) type DOT<T> =
    <<T as collateral::Config>::DOT as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub(crate) type PolkaBTC<T> = <<T as treasury::Config>::PolkaBTC as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub(crate) type UnsignedFixedPoint<T> = <T as Config>::UnsignedFixedPoint;

// TODO: concrete type is the same, circumvent this conversion
pub(crate) type Inner<T> = <<T as Config>::UnsignedFixedPoint as FixedPointNumber>::Inner;
