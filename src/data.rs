use std::sync::Arc;
use std::rc::Rc;

/// Set of common operations on shared data.
///
/// The `Data` trait represents a set of common operations that can be performed on shared data,
/// regardless of what properties the underlying storage type has. These properties include, but
/// are not limited to, synchronization for multithreaded access and the ability to make copies of
/// the data.
///
/// The `Data` trait is generic over the type of the inner value of the shared data, `T`. It also
/// defines an associated type `Storage`, which represents the specific type that the inner value of
/// the shared data is wrapped in.
pub trait Data<T> {
    /// Underlying storage of the data.
    ///
    /// The type of this depends on what storage mechanism is being used.
    ///
    /// Usually, this is some wrapper type such as [`Arc<T>`] or [`Rc<T>`] that can be cheaply
    /// cloned.
    type Storage;

    /// Create some new wrapped data.
    ///
    /// # Examples
    ///
    /// Create new shared, threadsafe data:
    ///
    /// ```rust
    /// use imstr::data::{Data, Threadsafe};
    ///
    /// let threadsafe = Threadsafe::new("Hello".to_string());
    /// ```
    ///
    /// Create new shared, local (non-threadsafe) data:
    ///
    /// ```rust
    /// use imstr::data::{Data, Local};
    ///
    /// let local = Local::new("Hello".to_string());
    /// ```
    fn new(value: T) -> Self;

    /// Returns an immutable reference to the shared data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use imstr::data::{Data, Local};
    ///
    /// let data = Local::new(15);
    /// assert_eq!(data.get(), &15);
    /// ```
    fn get(&self) -> &T;

    /// Returns a mutable reference to the shared data.
    ///
    /// # Option
    ///
    /// Depending on the underlying storage type, it might not always be possible to get a mutable
    /// reference. For this reason, this method returns an `Option`. Generally, it should not be
    /// expected that it is possible to get a mutable reference since mutation is typically not
    /// possible for shared data. However, in the case that it is possible, this is a good
    /// optimisation because it means the data does not have to be copied in order to mutate it.
    ///
    /// For example, [`Arc`](std::sync::Arc), which backs the [`Threadsafe`] data type,  allows
    /// mutating the data if and only if there is only a single reference to it alive (the
    /// reference count is 1). The same is true for [`Rc`](std::rc::Rc) which backs the [`Local`]
    /// data type.
    ///
    /// # Examples
    ///
    /// Mutating some [`Local`] data:
    ///
    /// ```rust
    /// use imstr::data::{Data, Local};
    ///
    /// let mut data = Local::new(15);
    /// if let Some(mut data) = data.get_mut() {
    ///     *data += 1;
    /// }
    ///
    /// assert_eq!(data.get(), &16);
    /// ```
    ///
    /// Mutating some [`Threadsafe`] data:
    ///
    /// ```rust
    /// use imstr::data::{Data, Threadsafe};
    ///
    /// let mut data = Threadsafe::new(15);
    /// if let Some(mut data) = data.get_mut() {
    ///     *data += 1;
    /// }
    ///
    /// assert_eq!(data.get(), &16);
    /// ```
    fn get_mut(&mut self) -> Option<&mut T>;

    /// Returns a clone of the raw, inner storage.
    fn raw(&self) -> Self::Storage;
}

/// Shared data, thread-safe.
///
/// The `Sync<T>` type represents shared data that can be safely accessed from multiple threads
/// concurrently. It is backed by an `Arc<T>`, which is an atomic reference-counted pointer that
/// allows multiple ownership of its inner value.
///
/// Since the `Sync<T>` type is backed by an `Arc<T>`, the inner value of the shared data can be
/// cloned efficiently using the `clone()` method. The `Sync<T>` type also implements various
/// traits such as `Clone`, `Debug`, `Hash`, `Eq`, `PartialEq`, `Ord`, and `PartialOrd`, making it
/// easy to use and interact with in many contexts.
///
/// Note that while the `Sync<T>` type ensures that the inner value of the shared data can be
/// safely accessed from multiple threads concurrently, it does not guarantee any particular order
/// in which threads will access the data. If thread synchronization is required, additional
/// mechanisms such as locks may need to be used in conjunction with `Sync<T>`.
///
/// ```rust
/// # use imstr::data::{Data, Threadsafe};
/// // Create a new `Threadsafe<String>` containing the value "Hello, world!".
/// let mut shared = Threadsafe::new("Hello, world!".to_string());
///
/// // Use Data::get() to get a &String
/// assert_eq!(shared.get(), "Hello, world!");
///
/// // Retrieve a mutable reference to the shared data using the `get_mut()` method and modify it.
/// if let Some(string) = shared.get_mut() {
///     string.push_str(" It's a beautiful day!");
/// }
///
/// // This only worked because the string has not been cloned yet.
/// assert_eq!(shared.get(), "Hello, world! It's a beautiful day!");
///
/// // Clone the shared data
/// let cloned = shared.clone();
/// assert_eq!(cloned, shared);
///
/// // Move the `Threadsafe<String>` to another thread and access the shared data concurrently.
/// let handle = std::thread::spawn(move || {
///     let mut cloned = cloned;
///     assert_eq!(cloned.get(), "Hello, world! It's a beautiful day!");
///     assert_eq!(cloned.get_mut(), None);
/// });
///
/// // Wait for the other thread to complete.
/// handle.join().unwrap();
/// ```
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Threadsafe<T>(Arc<T>);

impl<T> Data<T> for Threadsafe<T> {
    type Storage = Arc<T>;

    fn new(value: T) -> Self {
        Threadsafe(Arc::new(value))
    }

    fn get(&self) -> &T {
        &self.0
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.0)
    }

    fn raw(&self) -> Self::Storage {
        self.0.clone()
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Local<T>(Rc<T>);

impl<T> Data<T> for Local<T> {
    type Storage = Rc<T>;

    fn new(value: T) -> Self {
        Local(Rc::new(value))
    }

    fn get(&self) -> &T {
        &self.0
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        Rc::get_mut(&mut self.0)
    }

    fn raw(&self) -> Self::Storage {
        self.0.clone()
    }
}

#[derive(Clone, Debug, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cloned<T>(T);

impl<T: Clone> Data<T> for Cloned<T> {
    type Storage = T;

    fn new(value: T) -> Self {
        Cloned(value)
    }

    fn get(&self) -> &T {
        &self.0
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        Some(&mut self.0)
    }

    fn raw(&self) -> Self::Storage {
        self.0.clone()
    }
}
