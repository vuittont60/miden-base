use core::cell::OnceCell;

use super::{
    Account, AccountDelta, AccountId, AccountStub, AdviceInputs, BlockHeader, InputNotes,
    OutputNotes, Program, TransactionId, TransactionInputs, TransactionOutputs, TransactionScript,
    TransactionWitness,
};

// EXECUTED TRANSACTION
// ================================================================================================

/// Describes the result of executing a transaction program for the Miden rollup.
///
/// Executed transaction serves two primary purposes:
/// - It contains a complete description of the effects of the transaction. Specifically, it
///   contains all output notes created as the result of the transaction and describes all the
///   changes made to the involved account (i.e., the account delta).
/// - It contains all the information required to re-execute and prove the transaction in a
///   stateless manner. This includes all public transaction inputs, but also all nondeterministic
///   inputs that the host provided to Miden VM while executing the transaction (i.e., advice
///   witness).
#[derive(Debug, Clone)]
pub struct ExecutedTransaction {
    id: OnceCell<TransactionId>,
    program: Program,
    tx_inputs: TransactionInputs,
    tx_outputs: TransactionOutputs,
    account_delta: AccountDelta,
    tx_script: Option<TransactionScript>,
    advice_witness: AdviceInputs,
}

impl ExecutedTransaction {
    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------

    /// Returns a new [ExecutedTransaction] instantiated from the provided data.
    ///
    /// # Panics
    /// Panics if input and output account IDs are not the same.
    pub fn new(
        program: Program,
        tx_inputs: TransactionInputs,
        tx_outputs: TransactionOutputs,
        account_delta: AccountDelta,
        tx_script: Option<TransactionScript>,
        advice_witness: AdviceInputs,
    ) -> Self {
        // make sure account IDs are consistent across transaction inputs and outputs
        assert_eq!(tx_inputs.account().id(), tx_outputs.account.id());

        Self {
            id: OnceCell::new(),
            program,
            tx_inputs,
            tx_outputs,
            account_delta,
            tx_script,
            advice_witness,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a unique identifier of this transaction.
    pub fn id(&self) -> TransactionId {
        *self.id.get_or_init(|| self.into())
    }

    /// Returns a reference the program defining this transaction.
    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Returns the ID of the account against which this transaction was executed.
    pub fn account_id(&self) -> AccountId {
        self.initial_account().id()
    }

    /// Returns the description of the account before the transaction was executed.
    pub fn initial_account(&self) -> &Account {
        self.tx_inputs.account()
    }

    /// Returns description of the account after the transaction was executed.
    pub fn final_account(&self) -> &AccountStub {
        &self.tx_outputs.account
    }

    /// Returns the notes consumed in this transaction.
    pub fn input_notes(&self) -> &InputNotes {
        self.tx_inputs.input_notes()
    }

    /// Returns the notes created in this transaction.
    pub fn output_notes(&self) -> &OutputNotes {
        &self.tx_outputs.output_notes
    }

    /// Returns a reference to the transaction script.
    pub fn tx_script(&self) -> Option<&TransactionScript> {
        self.tx_script.as_ref()
    }

    /// Returns the block header for the block against which the transaction was executed.
    pub fn block_header(&self) -> &BlockHeader {
        self.tx_inputs.block_header()
    }

    /// Returns a description of changes between the initial and final account states.
    pub fn account_delta(&self) -> &AccountDelta {
        &self.account_delta
    }

    /// Returns a reference to the inputs for this transaction.
    pub fn tx_inputs(&self) -> &TransactionInputs {
        &self.tx_inputs
    }

    /// Returns all the data requested by the VM from the advice provider while executing the
    /// transaction program.
    pub fn advice_witness(&self) -> &AdviceInputs {
        &self.advice_witness
    }

    // CONVERSIONS
    // --------------------------------------------------------------------------------------------

    /// Returns individual components of this transaction.
    pub fn into_parts(self) -> (AccountDelta, TransactionOutputs, TransactionWitness) {
        let tx_witness = TransactionWitness::new(
            self.program,
            self.tx_inputs,
            self.tx_script,
            self.advice_witness,
        );

        (self.account_delta, self.tx_outputs, tx_witness)
    }
}

impl From<ExecutedTransaction> for TransactionWitness {
    fn from(tx: ExecutedTransaction) -> Self {
        let (_, _, tx_witness) = tx.into_parts();
        tx_witness
    }
}
