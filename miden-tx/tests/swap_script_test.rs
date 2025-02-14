use common::{
    get_account_with_default_account_code, get_new_key_pair_with_advice_map, MockDataStore,
};
use miden_lib::notes::{create_swap_note, utils::build_p2id_recipient};
use miden_objects::{
    accounts::{Account, AccountId, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN},
    assembly::ProgramAst,
    assets::{Asset, AssetVault, FungibleAsset, NonFungibleAsset, NonFungibleAssetDetails},
    crypto::rand::RpoRandomCoin,
    notes::{NoteAssets, NoteMetadata},
    transaction::OutputNote,
    Felt,
};
use miden_tx::TransactionExecutor;
use mock::constants::{
    ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
    ACCOUNT_ID_SENDER,
};

mod common;

#[test]
fn test_swap_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let non_fungible_asset: Asset = NonFungibleAsset::new(
        &NonFungibleAssetDetails::new(faucet_id_2, vec![1, 2, 3, 4]).unwrap(),
    )
    .unwrap()
    .into();

    // Create sender and target account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_sk_felt) = get_new_key_pair_with_advice_map();
    let target_account = get_account_with_default_account_code(
        target_account_id,
        target_pub_key,
        Some(non_fungible_asset),
    );

    // Create the note containing the SWAP script
    let (note, repay_serial_num) = create_swap_note(
        sender_account_id,
        fungible_asset,
        non_fungible_asset,
        RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // CONSTRUCT AND EXECUTE TX (Success)
    // --------------------------------------------------------------------------------------------
    let data_store =
        MockDataStore::with_existing(Some(target_account.clone()), Some(vec![note.clone()]));

    let mut executor = TransactionExecutor::new(data_store.clone());
    executor.load_account(target_account_id).unwrap();

    let block_ref = data_store.block_header.block_num();
    let note_ids = data_store.notes.iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_code = ProgramAst::parse(
        "
            use.miden::contracts::auth::basic->auth_tx

            begin
                call.auth_tx::auth_tx_rpo_falcon512
            end
            ",
    )
    .unwrap();
    let tx_script_target = executor
        .compile_tx_script(tx_script_code.clone(), vec![(target_pub_key, target_sk_felt)], vec![])
        .unwrap();

    // Execute the transaction
    let transaction_result = executor
        .execute_transaction(target_account_id, block_ref, &note_ids, Some(tx_script_target))
        .unwrap();

    // target account vault delta
    let target_account_after: Account = Account::new(
        target_account.id(),
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );

    // Check that the target account has received the asset from the note
    assert_eq!(transaction_result.final_account().hash(), target_account_after.hash());

    // Check if only one `Note` has been created
    assert_eq!(transaction_result.output_notes().num_notes(), 1);

    // Check if the created `Note` is what we expect
    let recipient = build_p2id_recipient(sender_account_id, repay_serial_num).unwrap();

    let note_metadata =
        NoteMetadata::new(target_account_id, sender_account_id.into(), Felt::new(1));

    let note_assets = NoteAssets::new(&[non_fungible_asset]).unwrap();

    let requested_note = OutputNote::new(recipient, note_assets, note_metadata);

    let created_note = transaction_result.output_notes().get_note(0);

    assert_eq!(created_note, &requested_note);
}
