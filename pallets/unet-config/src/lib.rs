#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;

pub use pallet::*;
use sp_runtime::{
	traits::{One, Zero},
	PerU16,
};
use unet_traits::{
	constants_types::{Balance, GlobalId, ACCURACY},
	time, CategoryData, NFTMetadata, UnetConfig,
};

mod benchmarking;
mod weights;
use crate::weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: weights::WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// no available id
		NoAvailableId,
		/// category not found
		CategoryNotFound,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// AddWhitelist \[who\]
		AddWhitelist(T::AccountId),
		/// RemoveWhitelist \[who\]
		RemoveWhitelist(T::AccountId),
		/// Created NFT common category. \[category_id\]
		CreatedCategory(GlobalId),
		/// Updated NFT common category. \[category_id\]
		UpdatedCategory(GlobalId),
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub royalties_rate: PerU16,
		pub platform_fee_rate: PerU16,
		pub max_commission_reward_rate: PerU16,
		pub min_commission_agent_deposit: Balance,
		pub min_order_deposit: Balance,
		pub auction_close_delay: BlockNumberFor<T>,
		pub white_list: Vec<T::AccountId>,
		pub category_list: Vec<NFTMetadata>,
		pub _phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				platform_fee_rate: PerU16::from_rational(1u32, 10000u32),
				royalties_rate: PerU16::from_percent(20),
				max_commission_reward_rate: PerU16::from_percent(100),
				min_commission_agent_deposit: ACCURACY,
				min_order_deposit: ACCURACY,
				auction_close_delay: time::MINUTES.into(),
				white_list: vec![],
				category_list: vec![b"saaaa1".to_vec(), b"saaaa2".to_vec(), b"saaaa3".to_vec()],
				_phantom: Default::default(),
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_runtime_upgrade() -> Weight {
			Weight::from_ref_time(0 as u64)
		}
		fn integrity_test() {}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			PlatformFeeRate::<T>::put(self.platform_fee_rate);
			RoyaltiesRate::<T>::put(self.royalties_rate);
			MaxCommissionRewardRate::<T>::put(self.max_commission_reward_rate);
			MinCommissionAgentDeposit::<T>::put(self.min_commission_agent_deposit);
			MinOrderDeposit::<T>::put(self.min_order_deposit);
			AuctionCloseDelay::<T>::put(self.auction_close_delay);
			for a in &self.white_list {
				<Pallet<T> as UnetConfig<T::AccountId, T::BlockNumber>>::do_add_whitelist(a);
			}
			for c in &self.category_list {
				<Pallet<T> as UnetConfig<T::AccountId, T::BlockNumber>>::do_create_category(
					c.clone(),
				)
				.unwrap();
			}
			IsWhiteListActivated::<T>::put(true);
		}
	}

	/// auction delay
	#[pallet::storage]
	#[pallet::getter(fn auction_close_delay)]
	pub type AuctionCloseDelay<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Whitelist for class creation
	#[pallet::storage]
	#[pallet::getter(fn account_whitelist)]
	pub type AccountWhitelist<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

	/// Royalties rate, which can be set by council or sudo.
	///
	/// **Incentive:** In order to reward creators of nfts.
	/// A small part of trading price will be paid to the nft creator.
	///
	/// Royalty = Price * `RoyaltiesRate`
	#[pallet::storage]
	#[pallet::getter(fn royalties_rate)]
	pub type RoyaltiesRate<T: Config> = StorageValue<_, PerU16, ValueQuery>;

	/// Platform fee rate for trading nfts.
	/// After deals, it will transfer a small amount of price into the treasury.
	///
	/// PlatformFee = Price * `PlatformFeeRate`
	#[pallet::storage]
	#[pallet::getter(fn platform_fee_rate)]
	pub type PlatformFeeRate<T: Config> = StorageValue<_, PerU16, ValueQuery>;

	/// max distribution reward
	///
	/// Reward = (Price - Royalty - PlatformFee) * `distributionReward`
	/// It will pay the `Reward` to the secondary retailer.
	#[pallet::storage]
	#[pallet::getter(fn max_commission_reward_rate)]
	pub type MaxCommissionRewardRate<T: Config> = StorageValue<_, PerU16, ValueQuery>;

	/// min reference deposit
	///
	/// The secondary retailer who will get reward from helping selling
	/// should keep at least `MinReferenceDeposit` balances.
	#[pallet::storage]
	#[pallet::getter(fn min_commission_agent_deposit)]
	pub type MinCommissionAgentDeposit<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// The lowest deposit every order should deposit.
	#[pallet::storage]
	#[pallet::getter(fn min_order_deposit)]
	pub type MinOrderDeposit<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// Next available global ID.
	#[pallet::storage]
	#[pallet::getter(fn next_id)]
	pub type NextId<T: Config> = StorageValue<_, GlobalId, ValueQuery>;

	/// The storage of categories.
	#[pallet::storage]
	#[pallet::getter(fn categories)]
	pub type Categories<T: Config> = StorageMap<_, Twox64Concat, GlobalId, CategoryData>;

	/// The storage of white list De/Active.
	#[pallet::storage]
	pub type IsWhiteListActivated<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// add an account into whitelist
		#[pallet::weight(T::WeightInfo::add_whitelist())]
		#[transactional]
		pub fn add_whitelist(
			origin: OriginFor<T>,
			who: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_add_whitelist(&who);
			Ok((Pays::No).into())
		}

		/// remove an account from whitelist
		#[pallet::weight(T::WeightInfo::remove_whitelist())]
		#[transactional]
		pub fn remove_whitelist(
			origin: OriginFor<T>,
			who: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			AccountWhitelist::<T>::remove(&who);
			Self::deposit_event(Event::RemoveWhitelist(who));
			Ok((Pays::No).into())
		}

		/// Create a common category for trading NFT.
		/// A Selling NFT should belong to a category.
		///
		/// - `metadata`: metadata
		#[pallet::weight(T::WeightInfo::create_category())]
		#[transactional]
		pub fn create_category(
			origin: OriginFor<T>,
			metadata: NFTMetadata,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_create_category(metadata)?;
			Ok((Pays::No).into())
		}

		/// Update a common category.
		///
		/// - `category_id`: category ID
		/// - `metadata`: metadata
		#[pallet::weight(T::WeightInfo::update_category())]
		#[transactional]
		pub fn update_category(
			origin: OriginFor<T>,
			category_id: GlobalId,
			metadata: NFTMetadata,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			if let Some(category) = Self::categories(category_id) {
				let info = CategoryData { metadata, count: category.count };
				Categories::<T>::insert(category_id, info);
				Self::deposit_event(Event::UpdatedCategory(category_id));
			}
			Ok((Pays::No).into())
		}

		#[pallet::weight(T::WeightInfo::update_auction_close_delay())]
		#[transactional]
		pub fn update_auction_close_delay(
			origin: OriginFor<T>,
			delay: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			AuctionCloseDelay::<T>::set(delay);
			Ok((Pays::No).into())
		}

		/// En/disable feature whitelist check
		#[pallet::weight(T::WeightInfo::en_disable_whitelist())]
		#[transactional]
		pub fn en_disable_whitelist(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			IsWhiteListActivated::<T>::put(!IsWhiteListActivated::<T>::get());
			Ok((Pays::No).into())
		}

		/// Set royalties rate
		#[pallet::weight((100_000, DispatchClass::Operational))]
		#[transactional]
		pub fn set_royalties_rate(
			origin: OriginFor<T>,
			royalties_rate: PerU16,
		) -> DispatchResultWithPostInfo {
		  	ensure_root(origin)?;
			RoyaltiesRate::<T>::put(royalties_rate);
			Ok((Pays::No).into())
		}

		/// Set platform fee rate
		#[pallet::weight((100_000, DispatchClass::Operational))]
		#[transactional]
		pub fn set_platform_fee_rate(
			origin: OriginFor<T>,
			platform_fee_rate: PerU16,
		) -> DispatchResultWithPostInfo {
		  	ensure_root(origin)?;
			PlatformFeeRate::<T>::put(platform_fee_rate);
			Ok((Pays::No).into())
		}
	}
}

impl<T: Config> UnetConfig<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
	fn auction_close_delay() -> BlockNumberFor<T> {
		AuctionCloseDelay::<T>::get()
	}

	fn is_in_whitelist(who: &T::AccountId) -> bool {
		Self::account_whitelist(who).is_some()
	}

	fn is_enable_whitelist() -> bool {
		IsWhiteListActivated::<T>::get()
	}

	fn get_min_order_deposit() -> Balance {
		Self::min_order_deposit()
	}

	fn get_then_inc_id() -> Result<GlobalId, DispatchError> {
		NextId::<T>::try_mutate(|id| -> Result<GlobalId, DispatchError> {
			let current_id = *id;
			*id = id.checked_add(GlobalId::one()).ok_or(Error::<T>::NoAvailableId)?;
			Ok(current_id)
		})
	}

	fn inc_count_in_category(category_id: GlobalId) -> DispatchResult {
		Categories::<T>::try_mutate(category_id, |maybe_category| -> DispatchResult {
			let category = maybe_category.as_mut().ok_or(Error::<T>::CategoryNotFound)?;
			category.count = category.count.saturating_add(One::one());
			Ok(())
		})
	}

	fn dec_count_in_category(category_id: GlobalId) -> DispatchResult {
		Categories::<T>::try_mutate(category_id, |maybe_category| -> DispatchResult {
			let category = maybe_category.as_mut().ok_or(Error::<T>::CategoryNotFound)?;
			category.count = category.count.saturating_sub(One::one());
			Ok(())
		})
	}

	fn do_add_whitelist(who: &T::AccountId) {
		AccountWhitelist::<T>::insert(&who, ());
		Self::deposit_event(Event::AddWhitelist(who.clone()));
	}

	fn do_create_category(metadata: NFTMetadata) -> DispatchResultWithPostInfo {
		let category_id = <Self as UnetConfig<T::AccountId, BlockNumberFor<T>>>::get_then_inc_id()?;

		let info = CategoryData { metadata, count: Zero::zero() };
		Categories::<T>::insert(category_id, info);

		Self::deposit_event(Event::CreatedCategory(category_id));
		Ok(().into())
	}

	fn peek_next_gid() -> GlobalId {
		Self::next_id()
	}

	fn get_royalties_rate() -> PerU16 {
		Self::royalties_rate()
	}

	fn get_platform_fee_rate() -> PerU16 {
		Self::platform_fee_rate()
	}

	fn get_max_commission_reward_rate() -> PerU16 {
		Self::max_commission_reward_rate()
	}

	fn get_min_commission_agent_deposit() -> Balance {
		Self::min_commission_agent_deposit()
	}
}
