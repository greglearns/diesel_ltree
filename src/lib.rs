#[macro_use]
extern crate diesel;

#[cfg(test)]
mod tests;

mod types {
    use diesel::query_builder::{AstPass, QueryFragment};
    use diesel::sql_types::Text;
    use diesel::{deserialize, AppearsOnTable, Expression, QueryResult, SelectableExpression};

    #[derive(SqlType, QueryId, FromSqlRow, Clone, Debug, PartialEq)]
    #[postgres(type_name = "ltree")]
    pub struct Ltree(pub String);

    impl Expression for Ltree {
        type SqlType = Ltree;
    }

    // // Commented out until Postgres supports binary-protocol for Ltree
    // // https://commitfest.postgresql.org/24/2242/
    // // https://github.com/npgsql/npgsql/issues/699
    // impl<DB> diesel::types::FromSql<Ltree /*via postres(type_name="ltree")*/, DB> for Ltree
    // /*this is the local ltree type*/
    // where
    //     String: diesel::types::FromSql<Text, DB>,
    //     DB: diesel::backend::Backend,
    //     DB: diesel::types::HasSqlType<Ltree>,
    // {
    //     fn from_sql(raw: Option<&DB::RawValue>) -> deserialize::Result<Self> {
    //         String::from_sql(raw).map(Ltree)
    //     }
    // }

    impl<DB> diesel::types::FromSql<diesel::sql_types::Text, DB> for Ltree
    where
        String: diesel::types::FromSql<Text, DB>,
        DB: diesel::backend::Backend,
        DB: diesel::types::HasSqlType<Text>,
    {
        fn from_sql(raw: Option<&DB::RawValue>) -> deserialize::Result<Self> {
            String::from_sql(raw).map(Ltree)
        }
    }

    impl<DB> QueryFragment<DB> for Ltree
    where
        DB: diesel::backend::Backend,
    {
        fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
            // can remove this function after ltree binary-protocol support is added by postgres
            out.push_bind_param::<diesel::sql_types::Text, _>(&self.0)?; // can (probably?) change diesel::sql_types::Text to diesel::sql_types::Ltree after Ltree is supported in binary protocol
            out.push_sql(&"::text::ltree"); // cast the text to an ltree in the query, so that the client can sent the ltree as text
            Ok(())
        }
    }
    impl<QS> SelectableExpression<QS> for Ltree {}
    impl<QS> AppearsOnTable<QS> for Ltree {}
    impl diesel::expression::NonAggregate for Ltree {}

    #[derive(SqlType, Clone, Copy, QueryId)]
    #[postgres(type_name = "lquery")]
    pub struct Lquery;

    #[derive(SqlType, Clone, Copy, QueryId)]
    #[postgres(type_name = "ltxtquery")]
    pub struct Ltxtquery;
}

mod functions {
    use diesel::sql_types::*;
    use types::*;

    sql_function!(fn subltree(ltree: Ltree, start: Int4, end: Int4) -> Ltree);
    sql_function!(fn subpath(ltree: Ltree, offset: Int4, len: Int4) -> Ltree);
    // sql_function!(fn subpath(ltree: Ltree, offset: Int4) -> Ltree);
    sql_function!(fn nlevel(ltree: Ltree) -> Int4);
    //sql_function!(fn index(a: Ltree, b: Ltree) -> Int4);
    sql_function!(fn index(a: Ltree, b: Ltree, offset: Int4) -> Int4);
    sql_function!(fn text2ltree(text: Text) -> Ltree);
    sql_function!(fn ltree2text(ltree: Ltree) -> Text);
    sql_function!(fn lca(ltrees: Array<Ltree>) -> Ltree);

    sql_function!(fn lquery(x: Text) -> Lquery);
    sql_function!(fn ltxtquery(x: Text) -> Ltxtquery);
}

mod dsl {
    use diesel::expression::{AsExpression, Expression};
    use diesel::sql_types::Array;
    use types::*;

    mod predicates {
        use diesel::pg::Pg;
        use types::*;

        diesel_infix_operator!(Contains, " @> ", backend: Pg);
        diesel_infix_operator!(ContainedBy, " <@ ", backend: Pg);
        diesel_infix_operator!(Matches, " ~ ", backend: Pg);
        diesel_infix_operator!(MatchesAny, " ? ", backend: Pg);
        diesel_infix_operator!(TMatches, " @ ", backend: Pg);
        diesel_infix_operator!(Concat, " || ", Ltree, backend: Pg);
        diesel_infix_operator!(FirstContains, " ?@> ", Ltree, backend: Pg);
        diesel_infix_operator!(FirstContainedBy, " ?<@ ", Ltree, backend: Pg);
        diesel_infix_operator!(FirstMatches, " ?~ ", Ltree, backend: Pg);
        diesel_infix_operator!(FirstTMatches, " ?@ ", Ltree, backend: Pg);
    }

    use self::predicates::*;

    pub trait LtreeExtensions: Expression<SqlType = Ltree> + Sized {
        fn contains<T: AsExpression<Ltree>>(self, other: T) -> Contains<Self, T::Expression> {
            Contains::new(self, other.as_expression())
        }

        fn contains_any<T: AsExpression<Array<Ltree>>>(
            self,
            other: T,
        ) -> Contains<Self, T::Expression> {
            Contains::new(self, other.as_expression())
        }

        fn contained_by<T: AsExpression<Ltree>>(
            self,
            other: T,
        ) -> ContainedBy<Self, T::Expression> {
            ContainedBy::new(self, other.as_expression())
        }

        fn contained_by_any<T: AsExpression<Array<Ltree>>>(
            self,
            other: T,
        ) -> ContainedBy<Self, T::Expression> {
            ContainedBy::new(self, other.as_expression())
        }

        fn matches<T: AsExpression<Lquery>>(self, other: T) -> Matches<Self, T::Expression> {
            Matches::new(self, other.as_expression())
        }

        fn matches_any<T: AsExpression<Array<Lquery>>>(
            self,
            other: T,
        ) -> MatchesAny<Self, T::Expression> {
            MatchesAny::new(self, other.as_expression())
        }

        fn tmatches<T: AsExpression<Ltxtquery>>(self, other: T) -> TMatches<Self, T::Expression> {
            TMatches::new(self, other.as_expression())
        }

        fn concat<T: AsExpression<Ltree>>(self, other: T) -> Concat<Self, T::Expression> {
            Concat::new(self, other.as_expression())
        }
    }

    pub trait LtreeArrayExtensions: Expression<SqlType = Array<Ltree>> + Sized {
        fn any_contains<T: AsExpression<Ltree>>(self, other: T) -> Contains<Self, T::Expression> {
            Contains::new(self, other.as_expression())
        }

        fn any_contained_by<T: AsExpression<Ltree>>(
            self,
            other: T,
        ) -> ContainedBy<Self, T::Expression> {
            ContainedBy::new(self, other.as_expression())
        }

        fn any_matches<T: AsExpression<Lquery>>(self, other: T) -> Matches<Self, T::Expression> {
            Matches::new(self, other.as_expression())
        }

        fn any_matches_any<T: AsExpression<Array<Lquery>>>(
            self,
            other: T,
        ) -> MatchesAny<Self, T::Expression> {
            MatchesAny::new(self, other.as_expression())
        }

        fn any_tmatches<T: AsExpression<Ltxtquery>>(
            self,
            other: T,
        ) -> TMatches<Self, T::Expression> {
            TMatches::new(self, other.as_expression())
        }

        fn first_contains<T: AsExpression<Ltree>>(
            self,
            other: T,
        ) -> FirstContains<Self, T::Expression> {
            FirstContains::new(self, other.as_expression())
        }

        fn first_contained_by<T: AsExpression<Ltree>>(
            self,
            other: T,
        ) -> FirstContainedBy<Self, T::Expression> {
            FirstContainedBy::new(self, other.as_expression())
        }

        fn first_matches<T: AsExpression<Lquery>>(
            self,
            other: T,
        ) -> FirstMatches<Self, T::Expression> {
            FirstMatches::new(self, other.as_expression())
        }

        fn first_tmatches<T: AsExpression<Ltxtquery>>(
            self,
            other: T,
        ) -> FirstTMatches<Self, T::Expression> {
            FirstTMatches::new(self, other.as_expression())
        }
    }

    pub trait LqueryExtensions: Expression<SqlType = Lquery> + Sized {
        fn matches<T: AsExpression<Ltree>>(self, other: T) -> Matches<Self, T::Expression> {
            Matches::new(self, other.as_expression())
        }

        fn matches_any<T: AsExpression<Array<Ltree>>>(
            self,
            other: T,
        ) -> Matches<Self, T::Expression> {
            Matches::new(self, other.as_expression())
        }
    }

    pub trait LqueryArrayExtensions: Expression<SqlType = Array<Lquery>> + Sized {
        fn any_matches<T: AsExpression<Ltree>>(self, other: T) -> MatchesAny<Self, T::Expression> {
            MatchesAny::new(self, other.as_expression())
        }

        fn any_matches_any<T: AsExpression<Array<Ltree>>>(
            self,
            other: T,
        ) -> MatchesAny<Self, T::Expression> {
            MatchesAny::new(self, other.as_expression())
        }
    }

    pub trait LtxtqueryExtensions: Expression<SqlType = Ltxtquery> + Sized {
        fn tmatches<T: AsExpression<Ltree>>(self, other: T) -> TMatches<Self, T::Expression> {
            TMatches::new(self, other.as_expression())
        }

        fn tmatches_any<T: AsExpression<Array<Ltree>>>(
            self,
            other: T,
        ) -> TMatches<Self, T::Expression> {
            TMatches::new(self, other.as_expression())
        }
    }

    impl<T: Expression<SqlType = Ltree>> LtreeExtensions for T {}
    impl<T: Expression<SqlType = Array<Ltree>>> LtreeArrayExtensions for T {}
    impl<T: Expression<SqlType = Lquery>> LqueryExtensions for T {}
    impl<T: Expression<SqlType = Array<Lquery>>> LqueryArrayExtensions for T {}
    impl<T: Expression<SqlType = Ltxtquery>> LtxtqueryExtensions for T {}
}

pub use self::dsl::*;
pub use self::functions::*;
pub use self::types::*;
