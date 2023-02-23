// don't hate me
type Balance = u32;
type LangError = u64;
type AccountId = String;

use scale::{Decode, DecodeLimit, Encode, Error as DecodeError};

// The code in this section is generated code. This is just one example how it could look like.
// I didn't spent too much time on this part. I mainly focused on the interface to pallet-contracts
// (the `Call` trait). Hence there is probably a lot of room for improvement.
// ---------------------------------------------------------------------------------------

// For each message and constructor of the contract we generate a module which contains
// a `call` function whose result can be passed to pallet-contracts. Scroll to the bottom
// of this file for an example how I intent to use it in substrate.

// For our current plan we only need the `call` (`new` for constructors) function.
// But being able to just decode the output will probably be handy for tests and such.

pub mod trigger {
    use super::*;

    type Output = ();

    /// The native_value is generated as first argument iff the function is payable.
    /// This is not part of the input to the contract but an argument to pallet-contracts.
    ///
    /// Arguments should all be references if not primitive.
    pub fn call(
        native_value: Balance,
        trigger_value: bool,
        msg: &str,
    ) -> impl Call<Output> + IsMessage {
        PayableCall {
            native_value,
            input_data: (0xDEADBEEF_u32, native_value, trigger_value, msg).encode(),
        }
    }

    /// In case we just want to decode output but not encode any input.
    pub fn output_decoder() -> impl OutputDecoder<Output> {
        GenericDecoder
    }

    /// Just a convenience function in case the trait is needed.
    pub fn decode_output<D: OutputDecoder<Output, Output = Output>>(
        output_data: &[u8],
        decode_complexity_limit: u32,
    ) -> Result<Output, DecodeError> {
        D::decode_output(output_data, decode_complexity_limit)
    }

    /// Just a convenience function in case the trait is needed.
    pub fn decode_output_unsafe_unbounded<D: OutputDecoder<Output, Output = Output>>(
        output_data: &[u8],
    ) -> Result<Output, DecodeError> {
        D::decode_output_unsafe_unbounded(output_data)
    }
}

pub mod transfer {
    use super::*;

    type Output = Result<(), ()>;

    /// The `amount` is encoded and passed to the contract. As oposed to native_valie
    /// Mixing in pallet-contracts arguments might be confusing. A builder pattern might
    /// solve this issue. But if it is only about the value I would say that having
    /// this concise API is worth more than the additional clarity from the builder.
    pub fn call(
        from: &AccountId,
        to: &AccountId,
        amount: &Balance,
    ) -> impl Call<Output> + IsMessage {
        UnpayableCall {
            input_data: (0x1BADB002_u32, from, to, amount).encode(),
        }
    }

    // Left out the decoder functions here as they are exactly the same as for the
    // other modules. They should still be generated.
}

// constructor. We probably want to group constructors and messages separately for better
// discoverability. Instead of having them flat side by side.
// There can't be a mixup due to marker trait. But still I think we can improve.
pub mod with_trigger_value {
    use super::*;

    type Output = ();

    /// This is a constructor. We call the function `new` instead of `call`.
    ///
    /// But apart from the name and the marker trait there is no difference.
    pub fn new(trigger_value: bool) -> impl Call<Output> + IsConstructor {
        UnpayableCall {
            input_data: (0xBAADF00D_u32, trigger_value).encode(),
        }
    }

    // Left out the decoder functions here as they are exactly the same as for the
    // other modules. They should still be generated.
}

// ---------------------------------------------------------------------------------------

// The following code is not generated but the same for every contract.
// Hence it should probably be defined somewhere in ink! and re-exported by the contract
// we are trying to depend upon.

pub trait Call<T> {
    type Decoder: OutputDecoder<T>;

    /// Not part of the input. Is passed to pallet-contracts.
    fn native_value(&self) -> Balance;

    /// The input data that should be passed as-is to the contract  
    fn into_input_data(self) -> Vec<u8>;
}

pub trait OutputDecoder<T> {
    type Output: Decode + DecodeLimit;

    /// Decode whatever is returned from the contract.
    ///
    /// Outer Error: The decoding of the result can of course fail.
    /// InnerError: The latest metadata version mandates that we always have a lang error
    ///
    /// The output from a contract is hostile. Hence we need to allow to limit the decoding
    /// complexity in case this is used in unmetered and or privileged code (runtime).
    fn decode_output(
        output_data: &[u8],
        decode_complexity_limit: u32,
    ) -> Result<Self::Output, DecodeError>;

    /// Same as `decode_output` but without limit in case the contract is trusted.
    fn decode_output_unsafe_unbounded(output_data: &[u8]) -> Result<Self::Output, DecodeError>;
}

/// Marker trait: The call is a message.
pub trait IsMessage {}

/// Marker trait: The call is a constructor.
pub trait IsConstructor {}

struct PayableCall {
    native_value: Balance,
    input_data: Vec<u8>,
}

impl<T> Call<T> for PayableCall
where
    Result<T, LangError>: Decode + DecodeLimit,
{
    type Decoder = GenericDecoder;

    fn native_value(&self) -> Balance {
        self.native_value
    }

    fn into_input_data(self) -> Vec<u8> {
        self.input_data
    }
}

// It is okay to implement both traits:  We just leave out the trait that does not
// apply from our `-> impl Call + Marker` return type.
impl IsMessage for PayableCall {}
impl IsConstructor for PayableCall {}

struct UnpayableCall {
    input_data: Vec<u8>,
}

impl<T> Call<T> for UnpayableCall
where
    Result<T, LangError>: Decode + DecodeLimit,
{
    type Decoder = GenericDecoder;

    fn native_value(&self) -> Balance {
        0
    }

    fn into_input_data(self) -> Vec<u8> {
        self.input_data
    }
}

impl IsMessage for UnpayableCall {}
impl IsConstructor for UnpayableCall {}

struct GenericDecoder;

impl<T> OutputDecoder<T> for GenericDecoder
where
    Result<T, LangError>: Decode + DecodeLimit,
{
    type Output = Result<T, LangError>;

    fn decode_output(
        mut output_data: &[u8],
        decode_complexity_limit: u32,
    ) -> Result<Self::Output, DecodeError> {
        // We are **not** using decode **all** to be consistent with cross contract calls.
        DecodeLimit::decode_with_depth_limit(decode_complexity_limit, &mut output_data)
    }

    /// Same as `decode_output` but without limit in case the contract is trusted.
    fn decode_output_unsafe_unbounded(mut output_data: &[u8]) -> Result<Self::Output, DecodeError> {
        Decode::decode(&mut output_data)
    }
}
// ---------------------------------------------------------------------------------------

// This is how the code depending on a contract (probably within pallet-contracts) could
// look like.

type OutputOf<C, T> = <<C as Call<T>>::Decoder as OutputDecoder<T>>::Output;

/// This function is what we would add in pallet-contracts.
///
/// This is schematic. The real function will have more arguments and will return more stuff.
pub fn call<T, C: Call<T> + IsMessage>(
    _contract_addr: AccountId,
    _gas_limit: u64,
    call: C,
) -> Result<OutputOf<C, T>, DecodeError> {
    // this is what we pass into the contract execution
    let _value = call.native_value();
    let _input = call.into_input_data();

    // here we would call into the contract and get the output
    let output = Vec::new();

    C::Decoder::decode_output(output.as_ref(), 255)
}

/// This function is what we would add in pallet-contracts.
///
/// This is schematic. The real function will have more arguments and will return more stuff.
pub fn instantiate<T, C: Call<T> + IsConstructor>(
    _gas_limit: u64,
    call: C,
) -> (AccountId, Result<OutputOf<C, T>, DecodeError>) {
    // this is what we pass into the contract execution
    let _value = call.native_value();
    let _input = call.into_input_data();

    // here we would call into the contract and get the output
    let output = Vec::new();

    (
        "my_contract".to_string(),
        C::Decoder::decode_output(output.as_ref(), 255),
    )
}

fn main() {
    let addr = instantiate(500, with_trigger_value::new(false)).0;
    call(
        addr,
        200,
        transfer::call(&"alice".to_string(), &"bob".to_string(), &9_000),
    )
    .unwrap();
}
