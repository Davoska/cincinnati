use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::fmt;

// `Box<Any>` is now `Box<dyn Any>`, but we can't change yet (minimum Rust)
#[allow(warnings)]
type AnyMap = HashMap<TypeId, Box<Any + Send + Sync>, BuildHasherDefault<IdHasher>>;

// With TypeIds as keys, there's no need to hash them. They are already hashes
// themselves, coming from the compiler. The IdHasher just holds the u64 of
// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}



/// A type map of protocol extensions.
///
/// `Extensions` can be used by `Request` and `Response` to store
/// extra data derived from the underlying protocol.
#[derive(Default)]
pub struct Extensions {
    // If extensions are never used, no need to carry around an empty HashMap.
    // That's 3 words. Instead, this is only 1 word.
    map: Option<Box<AnyMap>>,
}

impl Extensions {
    /// Create an empty `Extensions`.
    #[inline]
    pub fn new() -> Extensions {
        Extensions {
            map: None,
        }
    }

    /// Insert a type into this `Extensions`.
    ///
    /// If a extension of this type already existed, it will
    /// be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// assert!(ext.insert(5i32).is_none());
    /// assert!(ext.insert(4u8).is_none());
    /// assert_eq!(ext.insert(9i32), Some(5i32));
    /// ```
    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) -> Option<T> {
        self
            .map
            .get_or_insert_with(|| Box::new(HashMap::default()))
            .insert(TypeId::of::<T>(), Box::new(val))
            .and_then(|boxed| {
                #[allow(warnings)]
                {
                (boxed as Box<Any + 'static>)
                    .downcast()
                    .ok()
                    .map(|boxed| *boxed)
                }
            })
    }

    /// Get a reference to a type previously inserted on this `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// assert!(ext.get::<i32>().is_none());
    /// ext.insert(5i32);
    ///
    /// assert_eq!(ext.get::<i32>(), Some(&5i32));
    /// ```
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self
            .map
            .as_ref()
            .and_then(|map| map.get(&TypeId::of::<T>()))
            .and_then(|boxed| {
                #[allow(warnings)]
                {
                (&**boxed as &(Any + 'static)).downcast_ref()
                }
            })
    }

    /// Get a mutable reference to a type previously inserted on this `Extensions`.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(String::from("Hello"));
    /// ext.get_mut::<String>().unwrap().push_str(" World");
    ///
    /// assert_eq!(ext.get::<String>().unwrap(), "Hello World");
    /// ```
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self
            .map
            .as_mut()
            .and_then(|map| map.get_mut(&TypeId::of::<T>()))
            .and_then(|boxed| {
                #[allow(warnings)]
                {
                (&mut **boxed as &mut (Any + 'static)).downcast_mut()
                }
            })
    }


    /// Remove a type from this `Extensions`.
    ///
    /// If a extension of this type existed, it will be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(5i32);
    /// assert_eq!(ext.remove::<i32>(), Some(5i32));
    /// assert!(ext.get::<i32>().is_none());
    /// ```
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self
            .map
            .as_mut()
            .and_then(|map| map.remove(&TypeId::of::<T>()))
            .and_then(|boxed| {
                #[allow(warnings)]
                {
                (boxed as Box<Any + 'static>)
                    .downcast()
                    .ok()
                    .map(|boxed| *boxed)
                }
            })
    }

    /// Clear the `Extensions` of all inserted extensions.
    ///
    /// # Example
    ///
    /// ```
    /// # use http::Extensions;
    /// let mut ext = Extensions::new();
    /// ext.insert(5i32);
    /// ext.clear();
    ///
    /// assert!(ext.get::<i32>().is_none());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        if let Some(ref mut map) = self.map {
            map.clear();
        }
    }
}

impl fmt::Debug for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Extensions")
            .finish()
    }
}

#[test]
fn test_extensions() {
    #[derive(Debug, PartialEq)]
    struct MyType(i32);

    let mut extensions = Extensions::new();

    extensions.insert(5i32);
    extensions.insert(MyType(10));

    assert_eq!(extensions.get(), Some(&5i32));
    assert_eq!(extensions.get_mut(), Some(&mut 5i32));

    assert_eq!(extensions.remove::<i32>(), Some(5i32));
    assert!(extensions.get::<i32>().is_none());

    assert_eq!(extensions.get::<bool>(), None);
    assert_eq!(extensions.get(), Some(&MyType(10)));
}
