#[cfg(feature = "std")]
pub use {std::boxed::Box, std::rc::Rc, std::sync::Arc};

#[cfg(not(feature = "std"))]
pub use {alloc::boxed::Box, alloc::rc::Rc, alloc::string::String, alloc::sync::Arc};

/// Set of common operations on shared data.
///
/// The `Data` trait represents a set of common operations that can be performed on shared data,
/// regardless of what properties the underlying storage type has. These properties include, but
/// are not limited to, synchronization for multithreaded access and the ability to make copies of
/// the data.
///
/// The `Data` trait is generic over the type of the inner value of the shared data, `T`.
/// Implementers of this trait must provide methods to immutable access, and may provide methods to
/// mutably access the data.
pub trait Data<T>: Clone {
    /// Create some new data.
    ///
    /// # Example
    ///
    /// Thread-safe shared data using [`Arc<T>`]:
    ///
    /// ```
    /// use imstr::data::Data;
    /// use std::sync::Arc;
    ///
    /// let data = <Arc<_> as Data<_>>::new(15);
    /// ```
    ///
    /// Non thread-safe shared data using [`Rc<T>`]:
    ///
    /// ```
    /// use imstr::data::Data;
    /// use std::rc::Rc;
    ///
    /// let data = <Rc<_> as Data<_>>::new(15);
    /// ```
    fn new(value: T) -> Self;

    /// Returns an immutable reference to the shared data.
    ///
    /// # Example
    ///
    /// Thread-safe shared data using [`Arc<T>`]:
    ///
    /// ```rust
    /// use imstr::data::Data;
    /// use std::sync::Arc;
    ///
    /// let data = Arc::new(15);
    /// assert_eq!(data.get(), &15);
    /// ```
    ///
    /// Non thread-safe shared data using [`Rc<T>`]:
    ///
    /// ```rust
    /// use imstr::data::Data;
    /// use std::rc::Rc;
    ///
    /// let data = Rc::new(15);
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
    /// For example, [`Arc`](std::sync::Arc), allows mutating the data if and only if there is only
    /// a single reference to it alive (the reference count is 1). The same is true for
    /// [`Rc`](std::rc::Rc).
    ///
    /// # Examples
    ///
    /// Mutating thread-safe data wrapped in [`Arc<T>`]:
    ///
    /// ```rust
    /// use imstr::data::Data;
    /// use std::sync::Arc;
    ///
    /// let mut data = Arc::new(15);
    /// if let Some(mut data) = data.get_mut() {
    ///     *data += 1;
    /// }
    ///
    /// assert_eq!(data.get(), &16);
    /// ```
    ///
    /// Mutating non-thread-safe data wrapped in [`Rc<T>`]:
    ///
    /// ```rust
    /// use imstr::data::Data;
    /// use std::rc::Rc;
    ///
    /// let mut data = Rc::new(15);
    /// if let Some(mut data) = data.get_mut() {
    ///     *data += 1;
    /// }
    ///
    /// assert_eq!(data.get(), &16);
    /// ```
    fn get_mut(&mut self) -> Option<&mut T>;
}

impl<T> Data<T> for Arc<T> {
    fn new(value: T) -> Self {
        Arc::new(value)
    }

    fn get(&self) -> &T {
        &self
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(self)
    }
}

impl<T> Data<T> for Rc<T> {
    fn new(value: T) -> Self {
        Rc::new(value)
    }

    fn get(&self) -> &T {
        &self
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        Rc::get_mut(self)
    }
}

impl<T: Clone> Data<T> for Box<T> {
    fn new(value: T) -> Self {
        Box::new(value)
    }

    fn get(&self) -> &T {
        &self
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        Some(self)
    }
}

/// Container for data which is not actually shared, but is cloned.
#[derive(Clone)]
pub struct Cloned<T>(T);

impl<T: Clone> Data<T> for Cloned<T> {
    fn new(value: T) -> Self {
        Cloned(value)
    }

    fn get(&self) -> &T {
        &self.0
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        Some(&mut self.0)
    }
}

#[cfg(test)]
fn test_i32<T: Data<i32>>() {
    let mut number = T::new(16);
    assert_eq!(number.get(), &16);
    if let Some(number) = number.get_mut() {
        *number += 4;
    }
    assert_eq!(number.get(), &20);
    let clone = number.clone();
    assert_eq!(clone.get(), number.get());
}

#[cfg(test)]
fn test_string<T: Data<String>>() {
    let mut string = T::new("Hello".into());
    assert_eq!(string.get(), "Hello");
    if let Some(string) = string.get_mut() {
        string.push_str(", World!");
    }
    assert_eq!(string.get(), "Hello, World!");
    let clone = string.clone();
    assert_eq!(clone.get(), string.get());
}

#[test]
fn test_all_i32() {
    test_i32::<Cloned<i32>>();
    test_i32::<Arc<i32>>();
    test_i32::<Rc<i32>>();
    test_i32::<Box<i32>>();

    test_string::<Cloned<String>>();
    test_string::<Arc<String>>();
    test_string::<Rc<String>>();
    test_string::<Box<String>>();
}
